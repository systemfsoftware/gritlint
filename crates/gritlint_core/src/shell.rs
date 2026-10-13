#![allow(clippy::disallowed_methods, clippy::disallowed_types)]

use crate::decode::{is_rule_file, is_scannable};
use crate::domain::{
    IgnoreGlobs, Pack, RawFile, RawTree, RelPath, RuleSource, SelectedFiles, SourcePath, Sources,
};
use crate::error::GritlintError;
use crate::lsp;
use crate::workspace::{self, Workspace, WorkspaceInputs};
use ignore::WalkBuilder;
use rayon::prelude::*;
use std::path::{Path, PathBuf};

pub fn read_rule_sources(paths: &[PathBuf]) -> Result<Vec<RuleSource>, GritlintError> {
    let mut sources = Vec::new();
    for path in paths {
        if path.is_dir() {
            collect_rules(path, &mut sources)?;
        } else if path.is_file() {
            sources.push(read_source(path)?);
        } else {
            return Err(GritlintError::RulePathMissing { path: path.clone() });
        }
    }
    Ok(sources)
}

pub fn walk(root: &Path, ignore: &IgnoreGlobs) -> Vec<PathBuf> {
    let walker = WalkBuilder::new(root)
        .standard_filters(true)
        .require_git(false)
        .git_ignore(true)
        .filter_entry(|entry| {
            let name = entry.file_name();
            name != "node_modules" && name != ".git"
        })
        .build();

    let mut paths: Vec<PathBuf> = walker
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
        .filter(|entry| {
            is_scannable(entry.path()) && !ignore.matches(&relative(root, entry.path()))
        })
        .map(ignore::DirEntry::into_path)
        .collect();
    paths.sort();
    paths
}

pub fn read_sources(root: &Path, paths: &[RelPath]) -> Result<Sources, GritlintError> {
    paths
        .par_iter()
        .map(|path| read_text(&root.join(path.as_str())).map(|text| (path.clone(), text)))
        .collect::<Vec<_>>()
        .into_iter()
        .collect()
}

pub fn read_tree(root: &Path) -> Result<RawTree, GritlintError> {
    let mut files = Vec::new();
    let mut directories = Vec::new();
    collect_tree(root, &mut files, &mut directories)?;
    files.sort_by(|left, right| left.path().cmp(right.path()));
    directories.sort();
    directories.dedup();
    Ok(RawTree::new(root.to_owned(), files, directories))
}

pub fn read_to_string(path: &Path) -> std::io::Result<String> {
    std::fs::read_to_string(path)
}

pub fn read_text(path: &Path) -> Result<String, GritlintError> {
    read_to_string(path).map_err(|source| GritlintError::Io {
        path: path.to_owned(),
        source,
    })
}

#[must_use]
pub fn is_file(path: &Path) -> bool {
    path.is_file()
}

pub fn absolute(path: &Path) -> Result<PathBuf, GritlintError> {
    std::fs::canonicalize(path).map_err(|source| GritlintError::Io {
        path: path.to_owned(),
        source,
    })
}

pub fn load_workspace(
    inputs: &WorkspaceInputs,
    packs: &[Pack],
) -> Result<Workspace, GritlintError> {
    let config_path = inputs.config_path();
    let config_text = if inputs.config.is_some() || is_file(&config_path) {
        Some(
            read_to_string(&config_path).map_err(|source| GritlintError::ConfigRead {
                path: config_path.clone(),
                source,
            })?,
        )
    } else {
        None
    };
    let configured = workspace::configure(inputs, config_text.as_deref())?;
    let rule_sources = read_rule_sources(configured.rule_paths())?;
    configured.load(packs, rule_sources)
}

pub fn select(workspace: &Workspace) -> Result<SelectedFiles, GritlintError> {
    workspace.select(walk(workspace.root(), workspace.ignore()))
}

pub struct Disk;

impl lsp::Files for Disk {
    fn load(&self, inputs: &WorkspaceInputs, packs: &[Pack]) -> Result<Workspace, GritlintError> {
        load_workspace(inputs, packs)
    }

    fn select(&self, workspace: &Workspace) -> Result<SelectedFiles, GritlintError> {
        select(workspace)
    }

    fn read(&self, root: &Path, paths: &[RelPath]) -> Result<Sources, GritlintError> {
        read_sources(root, paths)
    }

    fn absolute(&self, path: &Path) -> Result<PathBuf, GritlintError> {
        absolute(path)
    }
}

fn read_source(path: &Path) -> Result<RuleSource, GritlintError> {
    let text = read_text(path)?;
    Ok(RuleSource::new(SourcePath::new(path.to_owned()), text))
}

fn collect_rules(dir: &Path, sources: &mut Vec<RuleSource>) -> Result<(), GritlintError> {
    let mut paths = entries(dir)?;
    paths.sort();
    for path in paths {
        if path.is_dir() {
            if path.file_name().is_none_or(|name| name != "fixtures") {
                collect_rules(&path, sources)?;
            }
        } else if is_rule_file(&path) {
            sources.push(read_source(&path)?);
        }
    }
    Ok(())
}

fn collect_tree(
    dir: &Path,
    files: &mut Vec<RawFile>,
    directories: &mut Vec<PathBuf>,
) -> Result<(), GritlintError> {
    directories.push(dir.to_owned());
    for path in entries(dir)? {
        if path.is_dir() {
            collect_tree(&path, files, directories)?;
        } else {
            files.push(RawFile::new(path.clone(), read_text(&path)?));
        }
    }
    Ok(())
}

fn entries(dir: &Path) -> Result<Vec<PathBuf>, GritlintError> {
    let entries = std::fs::read_dir(dir).map_err(|source| GritlintError::Io {
        path: dir.to_owned(),
        source,
    })?;
    entries
        .map(|entry| {
            entry
                .map(|entry| entry.path())
                .map_err(|source| GritlintError::Io {
                    path: dir.to_owned(),
                    source,
                })
        })
        .collect()
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
