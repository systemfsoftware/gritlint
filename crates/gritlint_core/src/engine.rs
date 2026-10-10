//! The one door to the embedded `GritQL` engine.
//!
//! Ground rules this module is the only place allowed to break: everything a run
//! may look at arrives in the [`CompileRequest`] or the files handed to
//! [`Compiled::execute`] — no `.grit` directory, no `$HOME`, no cwd walk-up, no
//! standard-library fetch — and the engine is pinned to one upstream rev with
//! every network-bearing feature off.
//!
//! # How pack parameters bind (and why)
//!
//! A bound parameter becomes a **generated pattern definition**: binding
//! `condition` to the pattern fragment `"browser"` adds a library file holding
//! `pattern condition() { "browser" }`, and a rule reaches that value by calling
//! `condition()` wherever a pattern is accepted. This is the generated prelude
//! arm of the platform plan's parameter decision, chosen over passing values as
//! arguments to a synthesized entry call because argument binding would force
//! this adapter to parse each body, choose its entry definition among however
//! many it declares, and map its declared parameter order onto the values — all
//! to express what a definition already expresses. The definition form keeps a
//! rule body plain `GritQL`, lets a value sit anywhere a pattern can (a `<:`
//! target, an argument to `contains`), and fails loudly: a rule that calls a
//! parameter the caller did not bind does not compile, rather than matching
//! nothing.

use grit_util::{AnalysisLogs, FileOrigin, Range as EngineRange};
use marzano_core::api::MatchResult;
use marzano_core::pattern_compiler::compiler::src_to_problem_libs;
use marzano_core::problem::Problem;
use marzano_language::language::MarzanoLanguage as _;
use marzano_util::rich_path::RichFile;
use marzano_util::runtime::ExecutionContext;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

pub use marzano_language::target_language::TargetLanguage;

const PARAMETER_FILE_PREFIX: &str = "gritlint_parameter_";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedPattern {
    pub name: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputFile {
    pub path: String,
    pub contents: String,
}

#[derive(Debug, Clone)]
pub struct CompileRequest {
    pub body: String,
    pub library: Vec<NamedPattern>,
    pub language: TargetLanguage,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct Compiled {
    problem: Problem,
    warnings: Vec<ParseDiagnostic>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatchRange {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub start_byte: u32,
    pub end_byte: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineMatch {
    pub path: String,
    pub ranges: Vec<MatchRange>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseDiagnostic {
    pub path: String,
    pub level: u16,
    pub message: String,
    pub line: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EngineOutcome {
    pub matches: Vec<EngineMatch>,
    pub diagnostics: Vec<ParseDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    Compile(String),
    Request(String),
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Compile(message) => write!(formatter, "gritql did not compile: {message}"),
            Self::Request(message) => write!(formatter, "invalid engine request: {message}"),
        }
    }
}

impl std::error::Error for EngineError {}

pub fn compile(request: &CompileRequest) -> Result<Compiled, EngineError> {
    let library = build_library(request)?;

    let compilation = src_to_problem_libs(
        request.body.clone(),
        &library,
        request.language,
        None,
        None,
        None,
        None,
    )
    .map_err(|error| EngineError::Compile(error.to_string()))?;

    let warnings = compilation
        .compilation_warnings
        .iter()
        .map(|warning| ParseDiagnostic {
            path: warning
                .file
                .as_ref()
                .map(|file| file.to_string_lossy().into_owned())
                .unwrap_or_default(),
            level: warning.level.unwrap_or(0),
            message: warning.message.clone(),
            line: warning.position.map_or(1, |position| position.line),
            column: warning.position.map_or(1, |position| position.column),
        })
        .collect();

    Ok(Compiled {
        problem: compilation.problem,
        warnings,
    })
}

impl Compiled {
    #[must_use]
    pub fn execute(&self, files: Vec<InputFile>) -> EngineOutcome {
        let mut outcome = EngineOutcome {
            matches: Vec::new(),
            diagnostics: self.warnings.clone(),
        };

        let context = ExecutionContext::default();
        let files: Vec<RichFile> = files
            .into_iter()
            .map(|file| RichFile::new(file.path, file.contents))
            .collect();

        for result in self.problem.execute_files(files, &context) {
            match result {
                MatchResult::AnalysisLog(log) => outcome.diagnostics.push(ParseDiagnostic {
                    path: log.file,
                    level: log.level,
                    message: log.message,
                    line: log.position.line,
                    column: log.position.column,
                }),
                MatchResult::Match(_)
                | MatchResult::Rewrite(_)
                | MatchResult::CreateFile(_)
                | MatchResult::RemoveFile(_) => outcome.matches.push(EngineMatch {
                    path: result.file_name().unwrap_or_default().to_owned(),
                    ranges: result
                        .get_ranges()
                        .map(|ranges| ranges.iter().map(match_range).collect())
                        .unwrap_or_default(),
                }),
                MatchResult::DoneFile(_)
                | MatchResult::InputFile(_)
                | MatchResult::PatternInfo(_)
                | MatchResult::AllDone(_) => {}
            }
        }

        outcome
    }
}

#[must_use]
pub fn parse(language: TargetLanguage, file: &InputFile) -> bool {
    let mut logs = AnalysisLogs::from(Vec::new());
    language
        .get_parser()
        .parse_file(
            &file.contents,
            Some(Path::new(&file.path)),
            &mut logs,
            FileOrigin::Fresh,
        )
        .is_some()
}

fn build_library(request: &CompileRequest) -> Result<BTreeMap<String, String>, EngineError> {
    let mut library: BTreeMap<String, String> = BTreeMap::new();

    for definition in &request.library {
        if request.parameters.contains_key(&definition.name) {
            return Err(EngineError::Request(format!(
                "`{}` is both a library pattern and a bound parameter",
                definition.name
            )));
        }
        if library
            .insert(format!("{}.grit", definition.name), definition.body.clone())
            .is_some()
        {
            return Err(EngineError::Request(format!(
                "`{}` is defined twice in the library",
                definition.name
            )));
        }
    }

    for (name, value) in &request.parameters {
        library.insert(
            format!("{PARAMETER_FILE_PREFIX}{name}.grit"),
            format!("pattern {name}() {{ {value} }}"),
        );
    }

    Ok(library)
}

fn match_range(range: &EngineRange) -> MatchRange {
    MatchRange {
        start_line: range.start.line,
        start_column: range.start.column,
        end_line: range.end.line,
        end_column: range.end.column,
        start_byte: range.start_byte,
        end_byte: range.end_byte,
    }
}
