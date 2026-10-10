use crate::domain::{Finding, FixtureOutcome, Location, Outcome, Verdict};
use schemars::{JsonSchema, Schema, schema_for};
use serde::Serialize;

pub const REPORT_SCHEMA_VERSION: u32 = 1;
pub const CONCISE_FINDINGS: usize = 20;
pub const CONCISE_LINE_BYTES: usize = 200;
pub const SUMMARY_FINDINGS: usize = 100;
const ELLIPSIS: &str = "…";

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[schemars(description = "The report `gritlint check --format json` prints.")]
struct JsonReport {
    #[schemars(extend("const" = REPORT_SCHEMA_VERSION))]
    #[schemars(description = "Raised when a change to this document is incompatible.")]
    schema_version: u32,
    findings: Vec<JsonFinding>,
    summary: JsonSummary,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct JsonFinding {
    #[schemars(description = "`<pack>/<rule>`, or `local/<rule>` for a consumer rule file.")]
    rule_id: String,
    #[schemars(
        description = "Lowercase kebab-case; stable for a rule, so an agent can act on it."
    )]
    reason_code: String,
    message: String,
    #[schemars(description = "One imperative sentence: what to change.")]
    next_action: String,
    location: JsonLocation,
}

#[derive(Serialize, JsonSchema)]
struct JsonLocation {
    #[schemars(description = "Relative to the scan root, `/`-separated.")]
    path: String,
    #[schemars(
        description = "The violating node as the engine reported it; null when the violation is an absent key, so the location is the file alone."
    )]
    range: Option<JsonRange>,
}

#[derive(Serialize, JsonSchema)]
struct JsonRange {
    start: JsonPosition,
    end: JsonPosition,
}

#[derive(Serialize, JsonSchema)]
#[schemars(description = "One-based, as GritQL reports positions.")]
struct JsonPosition {
    line: u32,
    column: u32,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct JsonSummary {
    files_selected: usize,
    findings: usize,
    rules: Vec<JsonRuleCount>,
    #[schemars(description = "Enabled rules that evaluated no file.")]
    zero_file_rules: Vec<String>,
}

#[derive(Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct JsonRuleCount {
    rule_id: String,
    files: usize,
}

#[must_use]
pub fn report_schema() -> Schema {
    schema_for!(JsonReport)
}

pub fn render_json(outcome: &Outcome) -> Result<String, serde_json::Error> {
    let report = JsonReport {
        schema_version: REPORT_SCHEMA_VERSION,
        findings: outcome
            .findings()
            .iter()
            .map(|finding| JsonFinding {
                rule_id: finding.rule_id().as_string(),
                reason_code: finding.guidance().reason_code().as_str().to_owned(),
                message: finding.message().to_owned(),
                next_action: finding.guidance().next_action().to_owned(),
                location: json_location(finding.location()),
            })
            .collect(),
        summary: JsonSummary {
            files_selected: outcome.files_selected(),
            findings: outcome.findings().len(),
            rules: outcome
                .counts()
                .iter()
                .map(|count| JsonRuleCount {
                    rule_id: count.rule_id().as_string(),
                    files: count.files(),
                })
                .collect(),
            zero_file_rules: outcome.zero_file_rules(),
        },
    };
    let mut rendered = serde_json::to_string_pretty(&report)?;
    rendered.push('\n');
    Ok(rendered)
}

fn json_location(location: &Location) -> JsonLocation {
    JsonLocation {
        path: location.path().as_str().to_owned(),
        range: location.range().map(|range| JsonRange {
            start: JsonPosition {
                line: range.start.line,
                column: range.start.column,
            },
            end: JsonPosition {
                line: range.end.line,
                column: range.end.column,
            },
        }),
    }
}

#[must_use]
pub fn render_human(outcome: &Outcome) -> String {
    let mut lines: Vec<String> = outcome
        .findings()
        .iter()
        .flat_map(|finding| {
            [
                format!(
                    "{}: {}: {}",
                    finding.location(),
                    finding.rule_id(),
                    finding.message()
                ),
                format!(
                    "  {}: {}",
                    finding.guidance().reason_code(),
                    finding.guidance().next_action()
                ),
            ]
        })
        .collect();
    lines.extend(
        outcome
            .counts()
            .iter()
            .map(|count| format!("{}: evaluated {} file(s)", count.rule_id(), count.files())),
    );
    lines.push(verdict_line(outcome));
    lines.join("\n") + "\n"
}

fn verdict_line(outcome: &Outcome) -> String {
    match outcome.verdict() {
        Verdict::Clean => {
            let zero = outcome.zero_file_rules();
            let zero = if zero.is_empty() {
                "none".to_owned()
            } else {
                zero.join(", ")
            };
            format!(
                "clean: {} file(s) selected; zero-file rules: {zero}",
                outcome.files_selected()
            )
        }
        Verdict::Findings | Verdict::Refused => format!(
            "{} finding(s) in {} file(s) selected",
            outcome.findings().len(),
            outcome.files_selected()
        ),
    }
}

#[must_use]
pub fn render_concise(outcome: &Outcome) -> String {
    let findings = outcome.findings();
    let mut lines: Vec<String> = findings
        .iter()
        .take(CONCISE_FINDINGS)
        .map(concise_line)
        .collect();
    let rest = findings.len().saturating_sub(CONCISE_FINDINGS);
    if rest > 0 {
        lines.push(format!(
            "{ELLIPSIS} and {rest} more; --format json lists every finding"
        ));
    }
    lines.push(cut_end(&verdict_line(outcome), CONCISE_LINE_BYTES));
    lines.join("\n") + "\n"
}

fn concise_line(finding: &Finding) -> String {
    let location = finding.location().to_string();
    let head = format!("{} {}", finding.rule_id(), finding.guidance().reason_code());
    let message = one_line(finding.message());
    let full = format!("{location} {head} {message}");
    if full.len() <= CONCISE_LINE_BYTES {
        return full;
    }
    let fixed = location.len() + 1 + head.len();
    if fixed + 1 + ELLIPSIS.len() < CONCISE_LINE_BYTES {
        let message = cut_end(&message, CONCISE_LINE_BYTES - fixed - 1);
        return format!("{location} {head} {message}");
    }
    let location = cut_start(&location, CONCISE_LINE_BYTES.saturating_sub(head.len() + 1));
    cut_end(&format!("{location} {head}"), CONCISE_LINE_BYTES)
}

fn one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn cut_end(text: &str, budget: usize) -> String {
    if text.len() <= budget {
        return text.to_owned();
    }
    let mut end = budget.saturating_sub(ELLIPSIS.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{ELLIPSIS}", text.get(..end).unwrap_or_default())
}

fn cut_start(text: &str, budget: usize) -> String {
    if text.len() <= budget {
        return text.to_owned();
    }
    let mut start = text.len() - budget.saturating_sub(ELLIPSIS.len());
    while !text.is_char_boundary(start) {
        start += 1;
    }
    format!("{ELLIPSIS}{}", text.get(start..).unwrap_or_default())
}

#[must_use]
pub fn render_github(outcome: &Outcome) -> String {
    outcome
        .findings()
        .iter()
        .map(|finding| {
            let location = finding.location();
            let mut properties = vec![format!(
                "file={}",
                escape_property(location.path().as_str())
            )];
            if let Some(range) = location.range() {
                properties.push(format!("line={}", range.start.line));
                properties.push(format!("col={}", range.start.column));
                properties.push(format!("endLine={}", range.end.line));
                properties.push(format!("endColumn={}", range.end.column));
            }
            properties.push(format!(
                "title={}",
                escape_property(&format!(
                    "{} ({})",
                    finding.rule_id(),
                    finding.guidance().reason_code()
                ))
            ));
            format!(
                "::error {}::{}\n",
                properties.join(","),
                escape_data(&format!(
                    "{}\n{}",
                    finding.message(),
                    finding.guidance().next_action()
                ))
            )
        })
        .collect::<Vec<_>>()
        .concat()
}

#[must_use]
pub fn escape_data(text: &str) -> String {
    text.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

#[must_use]
pub fn escape_property(text: &str) -> String {
    escape_data(text).replace(':', "%3A").replace(',', "%2C")
}

#[must_use]
pub fn render_github_summary(outcome: &Outcome) -> String {
    let findings = outcome.findings();
    let mut parts = vec![format!("### gritlint\n\n{}\n", verdict_line(outcome))];
    if !findings.is_empty() {
        parts.push(
            "\n| Location | Rule | Reason | Message | Next action |\n| --- | --- | --- | --- | --- |\n"
                .to_owned(),
        );
        parts.extend(findings.iter().take(SUMMARY_FINDINGS).map(|finding| {
            format!(
                "| {} | {} | {} | {} | {} |\n",
                table_cell(&finding.location().to_string()),
                table_cell(&finding.rule_id().as_string()),
                table_cell(finding.guidance().reason_code().as_str()),
                table_cell(finding.message()),
                table_cell(finding.guidance().next_action()),
            )
        }));
        let rest = findings.len().saturating_sub(SUMMARY_FINDINGS);
        if rest > 0 {
            parts.push(format!(
                "\n{ELLIPSIS} and {rest} more; `--format json` lists every finding.\n"
            ));
        }
    }
    if !outcome.counts().is_empty() {
        parts.push("\n| Rule | Files evaluated |\n| --- | --- |\n".to_owned());
        parts.extend(outcome.counts().iter().map(|count| {
            format!(
                "| {} | {} |\n",
                table_cell(&count.rule_id().as_string()),
                count.files()
            )
        }));
    }
    parts.concat()
}

fn table_cell(text: &str) -> String {
    one_line(text).replace('\\', "\\\\").replace('|', "\\|")
}

#[must_use]
pub fn render_fixtures(outcome: &FixtureOutcome) -> String {
    let mut lines: Vec<String> = outcome
        .results()
        .iter()
        .map(|result| {
            let state = if result.passed() { "ok" } else { "FAIL" };
            let kind = result.kind().directory();
            if result.passed() {
                format!("{state} {kind} {} {}", result.rule_id(), result.case())
            } else {
                format!(
                    "{state} {kind} {} {}: {}",
                    result.rule_id(),
                    result.case(),
                    result.detail()
                )
            }
        })
        .collect();
    let passed = outcome.passed();
    let count = if passed {
        outcome.results().len()
    } else {
        outcome.failures().len()
    };
    let verdict = if passed { "passed" } else { "failed" };
    lines.push(format!("{count} case(s) {verdict}"));
    lines.join("\n") + "\n"
}
