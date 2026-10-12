use crate::domain::{ExportRefusal, ParamNameRefusal, RelPathRefusal};
use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum GritlintError {
    #[error("cannot read `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot read config `{path}`: {source}")]
    ConfigRead {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse config `{path}`: {source}")]
    ConfigParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("pack `{name}` is not known to gritlint (config `{config}`); known packs: {known}")]
    UnknownPack {
        name: String,
        config: PathBuf,
        known: String,
    },
    #[error("pack `{pack}` needs parameter `{param}` bound in `{config}`")]
    MissingPackParam {
        pack: String,
        param: String,
        config: PathBuf,
    },
    #[error("pack parameter `{name}` in `{config}` is not usable: {reason}")]
    InvalidParamName {
        name: String,
        config: PathBuf,
        reason: ParamNameRefusal,
    },
    #[error("rule path `{path}` does not exist")]
    RulePathMissing { path: PathBuf },
    #[error("rule `{path}` carries no single ```grit block")]
    RuleNoGritBlock { path: PathBuf },
    #[error(
        "rule `{path}` declares no GritQL language; add e.g. `language json` inside its grit block"
    )]
    RuleNoLanguage { path: PathBuf },
    #[error("rule `{path}`: frontmatter key `{key}` {problem}")]
    RuleFrontmatter {
        path: PathBuf,
        key: String,
        problem: FrontmatterProblem,
    },
    #[error("rule `{path}` opens a `---` frontmatter block and never closes it")]
    RuleFrontmatterUnclosed { path: PathBuf },
    #[error("rule id `{id}` is defined by both `{first}` and `{second}`")]
    RuleCollision {
        id: String,
        first: PathBuf,
        second: PathBuf,
    },
    #[error("rule `{id}` ({path}) does not compile: {message}")]
    RuleCompile {
        id: String,
        path: PathBuf,
        message: String,
    },
    #[error("no rules are enabled: `{config}` enables no packs and no rule path was given")]
    NoRulesEnabled { config: PathBuf },
    #[error("no rules or packs were found in `{path}`")]
    NoRulesFound { path: PathBuf },
    #[error("the scan root `{root}` selected no files")]
    NoFilesSelected { root: PathBuf },
    #[error("path `{path}` is not a usable relative path: {reason}")]
    RelPath {
        path: String,
        reason: RelPathRefusal,
    },
    #[error(
        "{file}:{line}:{column}: {TARGET_DOES_NOT_PARSE}: {message} Next action: fix the syntax at that position; if the language's own compiler accepts it, report the construct to gritlint, and never ignore the file."
    )]
    TargetParse {
        file: String,
        line: u32,
        column: u32,
        message: String,
    },
    #[error(
        "{file}: {TARGET_TOO_LARGE}: {message} Next action: the engine never scans a file over its size limit (1,000,000 bytes); if the file is generated or vendored, add it to `ignore` in the gritlint config, otherwise split it below the limit."
    )]
    TargetTooLarge { file: String, message: String },
    #[error(
        "{file}: {TARGET_NOT_SCANNED}: {message} Next action: report the message and the file to gritlint; the engine could not scan the file, so the run cannot pass it."
    )]
    TargetNotScanned { file: String, message: String },
    #[error(
        "{file}: {TARGET_UNREADABLE}: {message} Next action: make the file readable UTF-8 text, or add it to `ignore` in the gritlint config; `gritlint check` refuses the run until then."
    )]
    TargetUnreadable { file: String, message: String },
    #[error("target file `{path}` was planned for a rule but never read")]
    SourceUnread { path: String },
    #[error("ignore glob `{glob}` is invalid: {message}")]
    InvalidIgnoreGlob { glob: String, message: String },
    #[error("cannot render the report: {message}")]
    Render { message: String },
    #[error("pack `{path}` has no `pack.json` manifest")]
    PackManifestMissing { path: PathBuf },
    #[error("cannot parse pack manifest `{path}`: {source}")]
    PackManifestParse {
        path: PathBuf,
        #[source]
        source: serde_json::Error,
    },
    #[error("{0}")]
    ExportRefused(ExportRefusal),
}

const TARGET_DOES_NOT_PARSE: &str = "target-does-not-parse";
const TARGET_TOO_LARGE: &str = "target-too-large";
const TARGET_NOT_SCANNED: &str = "target-not-scanned";
const TARGET_UNREADABLE: &str = "target-unreadable";

/// Why the engine refused one target file. Only a parse failure has a position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetReason {
    DoesNotParse { line: u32, column: u32 },
    TooLarge,
    NotScanned,
    Unreadable,
}

impl TargetReason {
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::DoesNotParse { .. } => TARGET_DOES_NOT_PARSE,
            Self::TooLarge => TARGET_TOO_LARGE,
            Self::NotScanned => TARGET_NOT_SCANNED,
            Self::Unreadable => TARGET_UNREADABLE,
        }
    }

    #[must_use]
    pub const fn position(self) -> Option<(u32, u32)> {
        match self {
            Self::DoesNotParse { line, column } => Some((line, column)),
            Self::TooLarge | Self::NotScanned | Self::Unreadable => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TargetRefusal<'a> {
    file: &'a str,
    reason: TargetReason,
}

impl<'a> TargetRefusal<'a> {
    #[must_use]
    pub const fn file(&self) -> &'a str {
        self.file
    }

    #[must_use]
    pub const fn reason(&self) -> TargetReason {
        self.reason
    }
}

impl GritlintError {
    #[must_use]
    pub fn target(&self) -> Option<TargetRefusal<'_>> {
        let refusal = |file, reason| Some(TargetRefusal { file, reason });
        if let Self::TargetParse {
            file, line, column, ..
        } = self
        {
            return refusal(
                file,
                TargetReason::DoesNotParse {
                    line: *line,
                    column: *column,
                },
            );
        }
        if let Self::TargetTooLarge { file, .. } = self {
            return refusal(file, TargetReason::TooLarge);
        }
        if let Self::TargetNotScanned { file, .. } = self {
            return refusal(file, TargetReason::NotScanned);
        }
        if let Self::TargetUnreadable { file, .. } = self {
            return refusal(file, TargetReason::Unreadable);
        }
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrontmatterProblem {
    Missing,
    Repeated,
    Unknown,
    Empty,
    NotKebabCase,
}

impl std::fmt::Display for FrontmatterProblem {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => formatter.write_str("is missing"),
            Self::Repeated => formatter.write_str("is repeated"),
            Self::Unknown => write!(
                formatter,
                "is unknown; the keys are `{}`",
                crate::decode::FRONTMATTER_KEYS.join("`, `")
            ),
            Self::Empty => formatter.write_str("has no value"),
            Self::NotKebabCase => {
                formatter.write_str("must be lowercase kebab-case, such as `missing-provenance`")
            }
        }
    }
}
