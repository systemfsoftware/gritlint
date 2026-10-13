use crate::decode::{self, GritlintConfig};
use crate::domain::{
    CompiledRule, IgnoreGlobs, NonEmpty, Pack, PackName, ParamName, RuleSource, SelectedFiles,
};
use crate::error::GritlintError;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct WorkspaceInputs {
    pub config: Option<PathBuf>,
    pub rules: Vec<PathBuf>,
    pub root: PathBuf,
}

impl WorkspaceInputs {
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.config
            .clone()
            .unwrap_or_else(|| self.root.join("gritlint.json"))
    }
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

    #[must_use]
    pub fn ignore(&self) -> &IgnoreGlobs {
        &self.ignore
    }

    pub fn select(&self, walked: Vec<PathBuf>) -> Result<SelectedFiles, GritlintError> {
        decode::select_files(walked, &self.root)
    }
}

pub struct Configured {
    root: PathBuf,
    config: GritlintConfig,
    config_path: PathBuf,
    enabled: BTreeMap<PackName, BTreeMap<ParamName, String>>,
    rule_paths: Vec<PathBuf>,
}

impl Configured {
    #[must_use]
    pub fn rule_paths(&self) -> &[PathBuf] {
        &self.rule_paths
    }

    pub fn load(
        self,
        packs: &[Pack],
        rule_sources: Vec<RuleSource>,
    ) -> Result<Workspace, GritlintError> {
        let rules = decode::load_rules(packs, &self.enabled, rule_sources, &self.config_path)?;
        let ignore = decode::ignore_globs(&self.config.ignore)?;
        Ok(Workspace {
            root: self.root,
            rules,
            ignore,
        })
    }
}

pub fn configure(
    inputs: &WorkspaceInputs,
    config_text: Option<&str>,
) -> Result<Configured, GritlintError> {
    let config_path = inputs.config_path();
    let config = match config_text {
        Some(text) => GritlintConfig::parse(text).map_err(|source| GritlintError::ConfigParse {
            path: config_path.clone(),
            source,
        })?,
        None => GritlintConfig::default(),
    };
    let mut rule_paths: Vec<PathBuf> = config
        .rules
        .iter()
        .map(|path| resolve_against(&config_path, path))
        .collect();
    rule_paths.extend(inputs.rules.iter().cloned());
    let enabled = decode::enabled_packs(&config, &config_path)?;
    Ok(Configured {
        root: inputs.root.clone(),
        config,
        config_path,
        enabled,
        rule_paths,
    })
}

fn resolve_against(config: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_owned()
    } else {
        config.parent().unwrap_or_else(|| Path::new(".")).join(path)
    }
}
