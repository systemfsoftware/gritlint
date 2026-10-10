use crate::domain::{
    CaseKind, CaseResult, CompiledRule, DirectoryBatch, Evaluation, Execution, Finding,
    FixtureCase, FixtureLayout, FixtureOutcome, FixtureRule, Location, NonEmpty, Outcome,
    ParsedRule, Range, RelPath, RuleCount, RuleKind, RuleStep, ScanPlan, SelectedFile,
    SelectedFiles, Sources, Verdict,
};
use crate::engine::{EngineMatch, EngineOutcome, InputFile};
use crate::error::GritlintError;
use rayon::prelude::*;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::LazyLock;

type Diagnostic = (String, String);

struct Unlocated<'a> {
    rule: &'a CompiledRule,
    directory: &'a DirectoryBatch,
    matched: NonEmpty<RelPath>,
    message: String,
}

struct TaskRun<'a> {
    findings: Vec<Finding>,
    unlocated: Option<Unlocated<'a>>,
    diagnostic: Option<Diagnostic>,
}

pub struct Matched<'a> {
    findings: Vec<Finding>,
    unlocated: Vec<Unlocated<'a>>,
    counts: Vec<RuleCount>,
    files_selected: usize,
}

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
    let matched = match_rules(plan, sources)?;
    locate(matched, sources)
}

pub fn match_rules<'a>(
    plan: &'a ScanPlan,
    sources: &Sources,
) -> Result<Matched<'a>, GritlintError> {
    let tasks: Vec<Task<'_>> = plan.steps.iter().flat_map(tasks_of).collect();
    let runs = tasks
        .par_iter()
        .map(|task| run_task(task, sources))
        .collect::<Vec<_>>()
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;
    let (findings, unlocated, diagnostic) = runs.into_iter().fold(
        (Vec::new(), Vec::new(), None),
        |(mut findings, mut unlocated, diagnostic), run| {
            findings.extend(run.findings);
            unlocated.extend(run.unlocated);
            (findings, unlocated, diagnostic.or(run.diagnostic))
        },
    );
    match diagnostic {
        Some((file, message)) => Err(GritlintError::TargetParse { file, message }),
        None => Ok(Matched {
            findings,
            unlocated,
            counts: plan.steps.iter().map(count_of).collect(),
            files_selected: plan.files_selected,
        }),
    }
}

pub fn locate(matched: Matched<'_>, sources: &Sources) -> Result<Outcome, GritlintError> {
    let Matched {
        mut findings,
        unlocated,
        counts,
        files_selected,
    } = matched;
    let located = unlocated
        .into_par_iter()
        .map(|unlocated| place(unlocated, sources))
        .collect::<Result<Vec<_>, _>>()?;
    findings.extend(located);
    Ok(canonicalize(findings, counts, files_selected))
}

fn place(unlocated: Unlocated<'_>, sources: &Sources) -> Result<Finding, GritlintError> {
    let Unlocated {
        rule,
        directory,
        matched,
        message,
    } = unlocated;
    let located = match rule.locator() {
        None => None,
        Some(locator) => {
            let key = rule.rule().language().key();
            let inputs = directory
                .files()
                .iter()
                .filter(|file| {
                    file.language().key() == key && matched.iter().any(|path| path == file.path())
                })
                .map(|file| read_input(file, sources))
                .collect::<Result<Vec<_>, _>>()?;
            locator.execute(inputs).matches.iter().try_fold(
                None,
                |best: Option<(RelPath, Option<Range>)>, found| {
                    let candidate = (
                        relpath(&found.path)?,
                        found.ranges.iter().map(Range::from).min(),
                    );
                    Ok::<_, GritlintError>(match best {
                        Some(best) if best.0 <= candidate.0 => Some(best),
                        _ => Some(candidate),
                    })
                },
            )?
        }
    };
    let location = match located {
        Some((path, range)) => Location::new(path, range),
        None => Location::new(matched.first().clone(), None),
    };
    Ok(Finding::new(
        rule.rule().id().clone(),
        location,
        message,
        rule.rule().guidance().clone(),
    ))
}

pub fn executions(plan: &ScanPlan, sources: &Sources) -> Result<Vec<Execution>, GritlintError> {
    plan.steps
        .iter()
        .flat_map(tasks_of)
        .map(|task| execution(&task, sources))
        .collect()
}

fn execution(task: &Task<'_>, sources: &Sources) -> Result<Execution, GritlintError> {
    let files = task
        .files()
        .iter()
        .map(|file| input_file(task, file, sources).map(|input| (input, task.reads(file))))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Execution::new(
        task.rule().rule().language().engine(),
        files,
    ))
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

fn run_task<'a>(task: &Task<'a>, sources: &Sources) -> Result<TaskRun<'a>, GritlintError> {
    let compiled = task.rule();
    let outcome = compiled
        .compiled()
        .execute(execution(task, sources)?.into_files());
    match task {
        Task::Single { .. } => Ok(TaskRun {
            findings: outcome
                .matches
                .iter()
                .try_fold(Vec::new(), |mut findings, matched| {
                    findings.extend(single_findings(compiled.rule(), matched)?);
                    Ok::<_, GritlintError>(findings)
                })?,
            unlocated: None,
            diagnostic: first_diagnostic(&outcome, task.files()),
        }),
        Task::Directory { rule, directory } => {
            let mut matched = outcome
                .matches
                .iter()
                .map(|matched| relpath(&matched.path))
                .collect::<Result<Vec<_>, _>>()?;
            matched.sort_unstable();
            matched.dedup();
            Ok(TaskRun {
                findings: Vec::new(),
                unlocated: NonEmpty::new(matched).map(|matched| Unlocated {
                    rule,
                    directory,
                    matched,
                    message: format!(
                        "{} (directory `{}`)",
                        rule.rule().message(),
                        directory.directory().map_or("", RelPath::as_str)
                    ),
                }),
                diagnostic: first_diagnostic(&outcome, directory.files()),
            })
        }
    }
}

fn first_diagnostic(outcome: &EngineOutcome, files: &[SelectedFile]) -> Option<Diagnostic> {
    outcome.diagnostics.iter().find_map(|diagnostic| {
        files
            .iter()
            .any(|file| file.path().as_str() == diagnostic.path)
            .then(|| (diagnostic.path.clone(), diagnostic.message.clone()))
    })
}

fn single_findings(
    rule: &ParsedRule,
    matched: &EngineMatch,
) -> Result<Vec<Finding>, GritlintError> {
    let path = relpath(&matched.path)?;
    let finding = |range: Option<Range>| {
        Finding::new(
            rule.id().clone(),
            Location::new(path.clone(), range),
            rule.message().to_owned(),
            rule.guidance().clone(),
        )
    };
    Ok(if matched.ranges.is_empty() {
        vec![finding(None)]
    } else {
        matched
            .ranges
            .iter()
            .map(|range| finding(Some(Range::from(range))))
            .collect()
    })
}

fn relpath(path: &str) -> Result<RelPath, GritlintError> {
    RelPath::new(path).map_err(|reason| GritlintError::RelPath {
        path: path.to_owned(),
        reason,
    })
}

fn input_file(
    task: &Task<'_>,
    file: &SelectedFile,
    sources: &Sources,
) -> Result<InputFile, GritlintError> {
    if task.reads(file) {
        read_input(file, sources)
    } else {
        Ok(InputFile {
            path: file.path().as_str().to_owned(),
            contents: String::new(),
        })
    }
}

fn read_input(file: &SelectedFile, sources: &Sources) -> Result<InputFile, GritlintError> {
    let contents = sources
        .text(file.path())
        .ok_or_else(|| GritlintError::SourceUnread {
            path: file.path().as_str().to_owned(),
        })?;
    Ok(InputFile {
        path: file.path().as_str().to_owned(),
        contents: contents.to_owned(),
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
    findings.sort_by_cached_key(|finding| {
        (
            finding.rule_id().as_string(),
            finding.location().clone(),
            finding.message().to_owned(),
        )
    });
    findings
}

fn canonicalize_counts(mut counts: Vec<RuleCount>) -> Vec<RuleCount> {
    counts.sort_by_key(|count| count.rule_id().as_string());
    counts
}

#[must_use]
pub fn expected_location_line(finding: &Finding) -> String {
    let location = finding.location();
    match location.range() {
        None => format!("{} {}", finding.rule_id(), location.path()),
        Some(range) => format!(
            "{} {}:{}:{}-{}:{}",
            finding.rule_id(),
            location.path(),
            range.start.line,
            range.start.column,
            range.end.line,
            range.end.column
        ),
    }
}

static ASSERTED_RANGE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r":\d+:\d+-\d+:\d+$").ok());

fn asserts_a_range(line: &str) -> bool {
    ASSERTED_RANGE
        .as_ref()
        .is_some_and(|range| range.is_match(line))
}

fn fixture_results(rule: &FixtureRule, base: &Path) -> Vec<Result<CaseResult, GritlintError>> {
    let mut results = Vec::new();
    results.extend(
        rule.bad
            .is_empty()
            .then(|| unasserted(rule, base, CaseKind::Bad, "no known-bad case")),
    );
    results.extend(
        rule.good
            .is_empty()
            .then(|| unasserted(rule, base, CaseKind::Good, "no known-good case")),
    );
    let locator_unasserted = rule.rule.locate().is_some()
        && !rule.bad.is_empty()
        && !rule.bad.iter().any(|case| {
            case.expected_locations
                .iter()
                .flatten()
                .any(|line| asserts_a_range(line))
        });
    results.extend(locator_unasserted.then(|| {
        unasserted(
            rule,
            base,
            CaseKind::Bad,
            "the rule declares `locate`, but no bad case's expected-locations.txt asserts a range",
        )
    }));
    results.extend(rule.bad.iter().map(|case| evaluate(rule, case, base)));
    results.extend(rule.good.iter().map(|case| evaluate(rule, case, base)));
    results
}

fn unasserted(
    rule: &FixtureRule,
    base: &Path,
    kind: CaseKind,
    detail: &str,
) -> Result<CaseResult, GritlintError> {
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
        detail.to_owned(),
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
    let detail = match (case.kind, outcome.verdict()) {
        (CaseKind::Bad, Verdict::Findings) => location_mismatch(&outcome, case),
        (CaseKind::Good, Verdict::Clean) => None,
        (CaseKind::Bad, Verdict::Clean | Verdict::Refused) => {
            Some("the known-bad case produced no finding".to_owned())
        }
        (CaseKind::Good, Verdict::Findings | Verdict::Refused) => Some(
            outcome
                .findings()
                .first()
                .map_or_else(String::new, |finding| {
                    format!(
                        "the known-good case produced {}: {}",
                        finding.location(),
                        finding.message()
                    )
                }),
        ),
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
        detail.is_none(),
        detail.unwrap_or_default(),
    ))
}

fn location_mismatch(outcome: &Outcome, case: &FixtureCase) -> Option<String> {
    let mut reported: Vec<String> = outcome
        .findings()
        .iter()
        .map(expected_location_line)
        .collect();
    reported.sort_unstable();
    match &case.expected_locations {
        None => Some(format!(
            "commits no expected-locations.txt; it reports: {}",
            reported.join("; ")
        )),
        Some(expected) => {
            let mut expected = expected.clone();
            expected.sort_unstable();
            (expected != reported).then(|| {
                format!(
                    "reported locations differ from expected-locations.txt: expected {}; reported {}",
                    expected.join("; "),
                    reported.join("; ")
                )
            })
        }
    }
}
