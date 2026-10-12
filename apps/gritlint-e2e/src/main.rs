#![allow(
    clippy::disallowed_methods,
    reason = "the journey is the consumer: it writes the tree it scans"
)]

mod lsp;
use serde_json::Value;
use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::path::Path;
use std::process::Command;

const REPOSITORY: &str = "git+https://github.com/acme/widgets.git";

const PLANTED: [(&str, &str); 7] = [
    (
        "gritlint.json",
        r#"{
  "packs": {
    "npm-provenance": { "repositoryUrl": "`\"git+https://github.com/acme/widgets.git\"`" },
    "typecheck-build-mode": {}
  }
}
"#,
    ),
    (
        "packages/widget/package.json",
        r#"{
  "name": "@acme/widget",
  "version": "1.0.0",
  "bugs": { "url": "https://github.com/acme/widgets/issues" },
  "repository": { "type": "git", "url": "git+https://github.com/acme/other.git" },
  "publishConfig": { "access": "public", "provenance": true }
}
"#,
    ),
    (
        "packages/gadget/package.json",
        r#"{
  "name": "@acme/gadget",
  "version": "1.0.0",
  "publishConfig": { "access": "public", "provenance": true }
}
"#,
    ),
    (
        "packages/app/package.json",
        r#"{
  "name": "@acme/app",
  "private": true,
  "scripts": { "typecheck": "tsc --noEmit --incremental" }
}
"#,
    ),
    (
        "packages/app/tsconfig.json",
        r#"{
  "files": [],
  "references": [{ "path": "./tsconfig.app.json" }]
}
"#,
    ),
    (
        "packages/app/tsconfig.app.json",
        r#"{
  "compilerOptions": { "composite": true },
  "include": ["src"]
}
"#,
    ),
    ("packages/app/src/index.ts", "export const app = 1\n"),
];

const EXPECTED: [(&str, &str, bool); 3] = [
    (
        "npm-provenance/repository-url",
        "packages/gadget/package.json",
        false,
    ),
    (
        "npm-provenance/repository-url",
        "packages/widget/package.json",
        true,
    ),
    (
        "typecheck-build-mode/typecheck-build-mode",
        "packages/app/package.json",
        true,
    ),
];

struct Report {
    exit: i32,
    findings: Vec<Value>,
}

struct Edit {
    path: String,
    start: usize,
    end: usize,
    text: String,
}

struct Failure(String);

impl fmt::Debug for Failure {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

fn main() -> Result<(), Failure> {
    journey().map_err(Failure)
}

fn journey() -> Result<(), String> {
    let mut args = std::env::args_os().skip(1);
    let (Some(gritlint), Some(root), None) = (args.next(), args.next(), args.next()) else {
        return Err("usage: gritlint-e2e <gritlint> <empty directory>".to_owned());
    };
    let (gritlint, root) = (Path::new(&gritlint), Path::new(&root));
    plant(root)?;

    let first = check(gritlint, root)?;
    if first.exit != 1 {
        return Err(format!("the planted tree exited {}, not 1", first.exit));
    }
    let seen: BTreeSet<(String, String, bool)> = first
        .findings
        .iter()
        .map(summary)
        .collect::<Result<_, _>>()?;
    let expected: BTreeSet<(String, String, bool)> = EXPECTED
        .iter()
        .map(|(rule, path, ranged)| ((*rule).to_owned(), (*path).to_owned(), *ranged))
        .collect();
    if seen != expected || first.findings.len() != EXPECTED.len() {
        return Err(format!("expected findings {expected:?}, read {seen:?}"));
    }
    lsp::journey(gritlint, root, &first.findings)?;
    let edits = first
        .findings
        .iter()
        .map(|finding| act(root, finding))
        .collect::<Result<Vec<_>, _>>()?;
    apply(root, edits)?;

    let second = check(gritlint, root)?;
    if second.exit != 0 || !second.findings.is_empty() {
        return Err(format!(
            "after acting on every finding the rerun exited {} with {:?}",
            second.exit, second.findings
        ));
    }
    Ok(())
}

fn plant(root: &Path) -> Result<(), String> {
    PLANTED.iter().try_for_each(|(path, contents)| {
        let target = root.join(path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|error| format!("create {path}'s parent: {error}"))?;
        }
        fs::write(&target, contents).map_err(|error| format!("write {path}: {error}"))
    })
}

fn check(gritlint: &Path, root: &Path) -> Result<Report, String> {
    let output = Command::new(gritlint)
        .args(["check", "--format", "json"])
        .current_dir(root)
        .output()
        .map_err(|error| format!("run {}: {error}", gritlint.display()))?;
    let exit = output
        .status
        .code()
        .ok_or("gritlint was killed by a signal")?;
    let stderr = String::from_utf8_lossy(&output.stderr);
    if exit == 2 {
        return Err(format!("gritlint refused the run: {stderr}"));
    }
    let report: Value = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("stdout is not JSON ({error}); stderr: {stderr}"))?;
    if report.get("schemaVersion").and_then(Value::as_u64) != Some(1) {
        return Err(format!("schemaVersion is not 1: {report}"));
    }
    let findings = report
        .get("findings")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("no findings array: {report}"))?
        .clone();
    if (exit == 1) == findings.is_empty() {
        return Err(format!(
            "exit {exit} disagrees with {} finding(s)",
            findings.len()
        ));
    }
    Ok(Report { exit, findings })
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| format!("no `{key}` string in {value}"))
}

fn range(finding: &Value) -> Option<&Value> {
    finding
        .get("location")
        .and_then(|location| location.get("range"))
        .filter(|range| !range.is_null())
}

fn summary(finding: &Value) -> Result<(String, String, bool), String> {
    text(finding, "nextAction")?;
    let location = finding
        .get("location")
        .ok_or_else(|| format!("no location in {finding}"))?;
    Ok((
        text(finding, "ruleId")?.to_owned(),
        text(location, "path")?.to_owned(),
        range(finding).is_some(),
    ))
}

fn act(root: &Path, finding: &Value) -> Result<Edit, String> {
    let path = text(finding.get("location").unwrap_or(&Value::Null), "path")?.to_owned();
    let contents = read(root, &path)?;
    let reported = range(finding);
    let (start, end) = if let Some(range) = reported {
        (
            offset(&contents, range, "start")?,
            offset(&contents, range, "end")?,
        )
    } else {
        let head = contents
            .find('{')
            .ok_or_else(|| format!("{path} has no object"))?
            + 1;
        (head, head)
    };
    let text = match (text(finding, "reasonCode")?, reported.is_some()) {
        ("repository-url-mismatch", true) => format!("\"url\": \"{REPOSITORY}\""),
        ("repository-url-mismatch", false) => {
            format!("\n  \"repository\": {{ \"type\": \"git\", \"url\": \"{REPOSITORY}\" }},")
        }
        ("typecheck-not-build-mode", true) => "\"typecheck\": \"tsc -b\"".to_owned(),
        (code, _) => return Err(format!("no action for reason code {code} at {path}")),
    };
    Ok(Edit {
        path,
        start,
        end,
        text,
    })
}

fn offset(contents: &str, range: &Value, end: &str) -> Result<usize, String> {
    let position = range
        .get(end)
        .ok_or_else(|| format!("no `{end}` in {range}"))?;
    let number = |key: &str| {
        position
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value > 0)
            .ok_or_else(|| format!("no one-based `{key}` in {position}"))
    };
    let (line, column) = (number("line")?, number("column")?);
    let mut lines = contents.split_inclusive('\n');
    let line_start: usize = lines.by_ref().take(line - 1).map(str::len).sum();
    let line_text = lines
        .next()
        .ok_or_else(|| format!("line {line} is past the end of the file"))?;
    if column > line_text.len() + 1 {
        return Err(format!("column {column} is past the end of line {line}"));
    }
    Ok(line_start + column - 1)
}

fn apply(root: &Path, mut edits: Vec<Edit>) -> Result<(), String> {
    edits.sort_by(|left, right| (&left.path, right.start).cmp(&(&right.path, left.start)));
    edits.iter().try_for_each(|edit| {
        let contents = read(root, &edit.path)?;
        let (Some(before), Some(after)) = (contents.get(..edit.start), contents.get(edit.end..))
        else {
            return Err(format!(
                "{}..{} is outside {}",
                edit.start, edit.end, edit.path
            ));
        };
        fs::write(
            root.join(&edit.path),
            format!("{before}{}{after}", edit.text),
        )
        .map_err(|error| format!("write {}: {error}", edit.path))
    })
}

fn read(root: &Path, path: &str) -> Result<String, String> {
    fs::read_to_string(root.join(path)).map_err(|error| format!("read {path}: {error}"))
}
