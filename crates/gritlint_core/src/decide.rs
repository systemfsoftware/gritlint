use crate::domain::{
    CaseKind, CaseResult, CompiledRule, DirectoryBatch, Evaluation, Finding, FixtureCase,
    FixtureLayout, FixtureOutcome, FixtureRule, NonEmpty, Outcome, ParsedRule, RelPath, RuleCount,
    RuleKind, RuleStep, ScanPlan, SelectedFile, SelectedFiles, Sources, Verdict,
};
use crate::engine::{EngineMatch, EngineOutcome, InputFile};
use crate::error::GritlintError;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::Path;

type Diagnostic = (String, String);
type TaskRun = (Vec<Finding>, Option<Diagnostic>);

#[must_use]
pub fn plan(rules: &NonEmpty<CompiledRule>, files: &SelectedFiles) -> ScanPlan {
    let steps = rules
        .iter()
        .map(|compiled| {
            let rule = compiled.rule();
            let key = rule.language().key();
            let batch: Vec<SelectedFile> = files.batch(key).cloned().collect();
            let evaluation = match rule.kind() {
                RuleKind::SingleFile => Evaluation::SingleFile { batch },
                RuleKind::MultiFile => {
                    let mut input = batch;
                    input.extend(participants(rule, files, key));
                    let directories = partition_multifile(rule, &input);
                    let evaluated = directories
                        .iter()
                        .map(|directory| directory.files().len())
                        .sum();
                    Evaluation::MultiFile {
                        directories,
                        evaluated,
                    }
                }
            };
            RuleStep {
                rule: compiled.clone(),
                evaluation,
            }
        })
        .collect();
    ScanPlan::new(steps, files.total())
}

#[must_use]
pub fn partition_multifile(rule: &ParsedRule, batch: &[SelectedFile]) -> Vec<DirectoryBatch> {
    batch
        .iter()
        .filter(|file| in_scope(rule, file))
        .fold(
            BTreeMap::<Option<RelPath>, Vec<SelectedFile>>::new(),
            |mut directories, file| {
                directories
                    .entry(file.path().parent())
                    .or_default()
                    .push(file.clone());
                directories
            },
        )
        .into_iter()
        .map(|(directory, files)| DirectoryBatch { directory, files })
        .collect()
}

#[must_use]
pub fn sources_needed(plan: &ScanPlan) -> Vec<RelPath> {
    let tasks: Vec<Task<'_>> = plan.steps.iter().flat_map(tasks_of).collect();
    let mut paths: Vec<&RelPath> = tasks
        .iter()
        .flat_map(|task| {
            task.files()
                .iter()
                .filter(|file| task.reads(file))
                .map(SelectedFile::path)
        })
        .collect();
    paths.sort_unstable();
    paths.dedup();
    paths.into_iter().cloned().collect()
}

pub fn scan(plan: &ScanPlan, sources: &Sources) -> Result<Outcome, GritlintError> {
    let tasks: Vec<Task<'_>> = plan.steps.iter().flat_map(tasks_of).collect();
    let runs = tasks
        .par_iter()
        .map(|task| run_task(task, sources))
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let counts = plan.steps.iter().map(count_of).collect();
    let (findings, diagnostic) = runs.into_iter().fold(
        (Vec::<Finding>::new(), None),
        |(mut findings, first), (task_findings, task_diagnostic)| {
            findings.extend(task_findings);
            (findings, first.or(task_diagnostic))
        },
    );
    match diagnostic {
        Some((file, message)) => Err(GritlintError::TargetParse { file, message }),
        None => Ok(canonicalize(findings, counts, plan.files_selected)),
    }
}

#[must_use]
pub fn canonicalize(
    findings: Vec<Finding>,
    counts: Vec<RuleCount>,
    files_selected: usize,
) -> Outcome {
    Outcome::new(
        canonicalize_findings(findings),
        canonicalize_counts(counts),
        files_selected,
    )
}

pub fn run_fixtures(layout: &FixtureLayout) -> Result<FixtureOutcome, GritlintError> {
    let mut results = layout
        .units
        .iter()
        .flat_map(|unit| {
            unit.rules
                .iter()
                .flat_map(|rule| fixture_results(rule, &unit.base))
        })
        .collect::<Result<Vec<_>, _>>()?;
    results.sort_by(|left, right| {
        (
            left.rule_id().as_string(),
            left.case().as_str().to_owned(),
            left.kind(),
        )
            .cmp(&(
                right.rule_id().as_string(),
                right.case().as_str().to_owned(),
                right.kind(),
            ))
    });
    Ok(FixtureOutcome::new(results))
}

#[must_use]
pub fn exit_code(verdict: Verdict) -> u8 {
    match verdict {
        Verdict::Refused => 2,
        Verdict::Findings => 1,
        Verdict::Clean => 0,
    }
}

enum Task<'a> {
    Single {
        rule: &'a CompiledRule,
        batch: &'a [SelectedFile],
    },
    Directory {
        rule: &'a CompiledRule,
        directory: &'a DirectoryBatch,
    },
}

impl Task<'_> {
    fn rule(&self) -> &CompiledRule {
        match self {
            Task::Single { rule, .. } | Task::Directory { rule, .. } => rule,
        }
    }

    fn files(&self) -> &[SelectedFile] {
        match self {
            Task::Single { batch, .. } => batch,
            Task::Directory { directory, .. } => directory.files(),
        }
    }

    fn reads(&self, file: &SelectedFile) -> bool {
        file.language().key() == self.rule().rule().language().key()
    }
}

fn tasks_of(step: &RuleStep) -> Vec<Task<'_>> {
    let rule = &step.rule;
    match &step.evaluation {
        Evaluation::SingleFile { batch } => vec![Task::Single { rule, batch }],
        Evaluation::MultiFile { directories, .. } => directories
            .iter()
            .map(|directory| Task::Directory { rule, directory })
            .collect(),
    }
}

fn count_of(step: &RuleStep) -> RuleCount {
    let evaluated = match &step.evaluation {
        Evaluation::SingleFile { batch } => batch.len(),
        Evaluation::MultiFile { evaluated, .. } => *evaluated,
    };
    RuleCount::new(step.rule.rule().id().clone(), evaluated)
}

fn run_task(task: &Task<'_>, sources: &Sources) -> Result<TaskRun, GritlintError> {
    let compiled = task.rule();
    let files = task.files();
    let inputs = files
        .iter()
        .map(|file| input_file(task, file, sources))
        .collect::<Result<Vec<_>, _>>()?;
    let outcome = compiled.compiled().execute(inputs);
    match task {
        Task::Single { .. } => single_findings(compiled.rule(), &outcome, files),
        Task::Directory { directory, .. } => {
            directory_findings(compiled.rule(), &outcome, directory)
        }
    }
}

fn single_findings(
    rule: &ParsedRule,
    outcome: &EngineOutcome,
    batch: &[SelectedFile],
) -> Result<TaskRun, GritlintError> {
    let findings = outcome
        .matches
        .iter()
        .map(|matched| finding_for(rule, matched))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((findings, first_diagnostic(outcome, batch)))
}

fn directory_findings(
    rule: &ParsedRule,
    outcome: &EngineOutcome,
    directory: &DirectoryBatch,
) -> Result<TaskRun, GritlintError> {
    let diagnostic = first_diagnostic(outcome, directory.files());
    let message = format!(
        "{} (directory `{}`)",
        rule.message(),
        directory.directory().map_or("", RelPath::as_str)
    );
    let finding = (!outcome.matches.is_empty())
        .then(|| first_match(outcome, directory.files()))
        .transpose()?
        .flatten()
        .map(|(path, line)| Finding::new(rule.id().clone(), path, line, message));
    Ok((finding.into_iter().collect(), diagnostic))
}

fn first_diagnostic(outcome: &EngineOutcome, files: &[SelectedFile]) -> Option<Diagnostic> {
    outcome.diagnostics.iter().find_map(|diagnostic| {
        files
            .iter()
            .any(|file| file.path().as_str() == diagnostic.path)
            .then(|| (diagnostic.path.clone(), diagnostic.message.clone()))
    })
}

fn first_match(
    outcome: &EngineOutcome,
    files: &[SelectedFile],
) -> Result<Option<(RelPath, u32)>, GritlintError> {
    let mut matched: Vec<&EngineMatch> = outcome.matches.iter().collect();
    matched.sort_by(|left, right| left.path.cmp(&right.path));
    let chosen = matched
        .iter()
        .find(|matched| !matched.ranges.is_empty())
        .or_else(|| matched.first());
    match chosen {
        Some(matched) => {
            let path = RelPath::new(&matched.path).map_err(|reason| GritlintError::RelPath {
                path: matched.path.clone(),
                reason,
            })?;
            Ok(Some((
                path,
                matched.ranges.first().map_or(1, |range| range.start_line),
            )))
        }
        None => Ok(files.first().map(|file| (file.path().clone(), 1))),
    }
}

fn finding_for(rule: &ParsedRule, matched: &EngineMatch) -> Result<Finding, GritlintError> {
    let path = RelPath::new(&matched.path).map_err(|reason| GritlintError::RelPath {
        path: matched.path.clone(),
        reason,
    })?;
    let line = matched.ranges.first().map_or(1, |range| range.start_line);
    Ok(Finding::new(
        rule.id().clone(),
        path,
        line,
        rule.message().to_owned(),
    ))
}

fn input_file(
    task: &Task<'_>,
    file: &SelectedFile,
    sources: &Sources,
) -> Result<InputFile, GritlintError> {
    let contents = if task.reads(file) {
        sources
            .text(file.path())
            .ok_or_else(|| GritlintError::SourceUnread {
                path: file.path().as_str().to_owned(),
            })?
            .to_owned()
    } else {
        String::new()
    };
    Ok(InputFile {
        path: file.path().as_str().to_owned(),
        contents,
    })
}

fn in_scope(rule: &ParsedRule, file: &SelectedFile) -> bool {
    rule.targets().is_empty()
        || rule
            .targets()
            .iter()
            .any(|target| target.is_match(file.path().as_str()))
}

fn participants(rule: &ParsedRule, files: &SelectedFiles, own_key: &str) -> Vec<SelectedFile> {
    files
        .language_keys()
        .filter(|key| *key != own_key)
        .flat_map(|key| files.batch(key))
        .filter(|file| {
            rule.targets()
                .iter()
                .any(|target| target.is_match(file.path().as_str()))
        })
        .map(|file| SelectedFile::new(file.language().clone(), file.path().clone()))
        .collect()
}

fn canonicalize_findings(mut findings: Vec<Finding>) -> Vec<Finding> {
    findings.sort_by(|left, right| {
        (
            left.rule_id().as_string(),
            left.path().as_str().to_owned(),
            left.line(),
            left.message().to_owned(),
        )
            .cmp(&(
                right.rule_id().as_string(),
                right.path().as_str().to_owned(),
                right.line(),
                right.message().to_owned(),
            ))
    });
    findings
}

fn canonicalize_counts(mut counts: Vec<RuleCount>) -> Vec<RuleCount> {
    counts.sort_by_key(|count| count.rule_id().as_string());
    counts
}

fn fixture_results(rule: &FixtureRule, base: &Path) -> Vec<Result<CaseResult, GritlintError>> {
    let mut results = Vec::new();
    results.extend(
        rule.bad
            .is_empty()
            .then(|| untested(rule, base, CaseKind::Bad)),
    );
    results.extend(
        rule.good
            .is_empty()
            .then(|| untested(rule, base, CaseKind::Good)),
    );
    results.extend(rule.bad.iter().map(|case| evaluate(rule, case, base)));
    results.extend(rule.good.iter().map(|case| evaluate(rule, case, base)));
    results
}

fn untested(rule: &FixtureRule, base: &Path, kind: CaseKind) -> Result<CaseResult, GritlintError> {
    let directory = kind.directory();
    let case =
        RelPath::from_root(base, &rule.fixture_directory.join(directory)).map_err(|reason| {
            GritlintError::RelPath {
                path: rule.fixture_directory.join(directory).display().to_string(),
                reason,
            }
        })?;
    Ok(CaseResult::new(
        rule.rule.id().clone(),
        case,
        kind,
        false,
        format!("no known-{directory} case"),
    ))
}

fn evaluate(
    rule: &FixtureRule,
    case: &FixtureCase,
    base: &Path,
) -> Result<CaseResult, GritlintError> {
    let mut scoped = rule.rule.clone();
    scoped.with_params(case.params.clone());
    let compiled = CompiledRule::compile(&scoped, &[&scoped])?;
    let rules = NonEmpty::from_head(compiled, Vec::new());
    let plan = plan(&rules, &case.files);
    let outcome = scan(&plan, &case.sources)?;
    let verdict = outcome.verdict();
    let passed = match case.kind {
        CaseKind::Bad => verdict == Verdict::Findings,
        CaseKind::Good => verdict == Verdict::Clean,
    };
    let detail = match (case.kind, verdict) {
        (CaseKind::Bad, Verdict::Findings) | (CaseKind::Good, Verdict::Clean) => String::new(),
        (CaseKind::Bad, Verdict::Clean | Verdict::Refused)
        | (CaseKind::Good, Verdict::Findings | Verdict::Refused) => describe(&outcome, case.kind),
    };
    let case_path =
        RelPath::from_root(base, &case.directory).map_err(|reason| GritlintError::RelPath {
            path: case.directory.display().to_string(),
            reason,
        })?;
    Ok(CaseResult::new(
        rule.rule.id().clone(),
        case_path,
        case.kind,
        passed,
        detail,
    ))
}

fn describe(outcome: &Outcome, kind: CaseKind) -> String {
    match kind {
        CaseKind::Bad => "the known-bad case produced no finding".to_owned(),
        CaseKind::Good => outcome
            .findings()
            .first()
            .map_or_else(String::new, |finding| {
                format!(
                    "the known-good case produced {}:{}: {}",
                    finding.path(),
                    finding.line(),
                    finding.message()
                )
            }),
    }
}
