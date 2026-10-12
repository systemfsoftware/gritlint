use crate::decode::{self, GritlintConfig};
use crate::domain::{CompiledRule, IgnoreGlobs, NonEmpty, Pack, SelectedFiles};
use crate::error::GritlintError;
use crate::shell;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorkspaceInputs {
    pub config: Option<PathBuf>,
    pub rules: Vec<PathBuf>,
    pub root: PathBuf,
}

pub struct Workspace {
    root: PathBuf,
    rules: NonEmpty<CompiledRule>,
    ignore: IgnoreGlobs,
}

impl Workspace {
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    #[must_use]
    pub fn rules(&self) -> &NonEmpty<CompiledRule> {
        &self.rules
    }

    pub fn select(&self) -> Result<SelectedFiles, GritlintError> {
        decode::select_files(shell::walk(&self.root, &self.ignore), &self.root)
    }
}

pub fn load(inputs: &WorkspaceInputs, packs: &[Pack]) -> Result<Workspace, GritlintError> {
    let config_path = inputs
        .config
        .clone()
        .unwrap_or_else(|| inputs.root.join("gritlint.json"));
    let config = if inputs.config.is_some() || shell::is_file(&config_path) {
        let text =
            shell::read_to_string(&config_path).map_err(|source| GritlintError::ConfigRead {
                path: config_path.clone(),
                source,
            })?;
        GritlintConfig::parse(&text).map_err(|source| GritlintError::ConfigParse {
            path: config_path.clone(),
            source,
        })?
    } else {
        GritlintConfig::default()
    };

    let mut consumer: Vec<PathBuf> = config
        .rules
        .iter()
        .map(|path| resolve_against(&config_path, path))
        .collect();
    consumer.extend(inputs.rules.iter().cloned());

    let enabled = decode::enabled_packs(&config, &config_path)?;
    let rule_sources = shell::read_rule_sources(&consumer)?;
    let rules = decode::load_rules(packs, &enabled, rule_sources, &config_path)?;
    let ignore = decode::ignore_globs(&config.ignore)?;
    Ok(Workspace {
        root: inputs.root.clone(),
        rules,
        ignore,
    })
}

fn resolve_against(config: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        config.parent().unwrap_or_else(|| Path::new(".")).join(path)
    }
}
