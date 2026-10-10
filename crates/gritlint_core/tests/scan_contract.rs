#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

use gritlint_core::GritlintError;
use gritlint_core::decide;
use gritlint_core::decode;
use gritlint_core::domain::{
    NonEmpty, Outcome, ParsedRule, Position, Range, RuleSource, ScanPlan, SourcePath, Sources,
};
use gritlint_core::encode;
use gritlint_core::shell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const SIBLING_JOIN: &str = r#"language json
multifile {
  bubble($dir) file($name, $body) where {
    $name <: r"(.*/)tsconfig\.json"($dir),
    $body <: contains `"compilerOptions"`
  },
  bubble($dir) file($name, $body) where {
    $name <: r"(.*/)package\.json"($package_dir),
    $package_dir <: $dir,
    $body <: contains `"vite": $_`
  }
}
"#;

const API_EXTRACTOR: &str = r#"language json
multifile {
  file($name, $body) where {
    $name <: r".*api-extractor\.json$"
  }
}
"#;

const REACT_PAIR: &str = "language json\n`\"react\": $_`\n";

fn write(base: &Path, path: &str, contents: &str) {
    let target = base.join(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("create the parent directory");
    }
    fs::write(target, contents).expect("write the file");
}

const FRONTMATTER: &str = "---\nreasonCode: test-reason\nnextAction: Fix it.\n---\n";

fn rule(name: &str, body: &str) -> ParsedRule {
    let source = RuleSource::new(
        SourcePath::new(PathBuf::from(format!("{name}.grit"))),
        format!("{FRONTMATTER}{body}"),
    );
    decode::parse_rule(None, &source).expect("parse the rule")
}

fn located_rule(name: &str, body: &str, locate: &str) -> ParsedRule {
    let source = RuleSource::new(
        SourcePath::new(PathBuf::from(format!("{name}.grit"))),
        format!("---\nreasonCode: test-reason\nnextAction: Fix it.\nlocate: {locate}\n---\n{body}"),
    );
    decode::parse_rule(None, &source).expect("parse the located rule")
}

fn markdown_rule(name: &str, description: &str, body: &str) -> ParsedRule {
    let source = RuleSource::new(
        SourcePath::new(PathBuf::from(format!("{name}.md"))),
        format!("{FRONTMATTER}\n# {name}\n\n{description}\n\n```grit\n{body}\n```\n"),
    );
    decode::parse_rule(None, &source).expect("parse the markdown rule")
}

fn plan(root: &Path, rules: &[ParsedRule], ignore: &[String]) -> Result<ScanPlan, GritlintError> {
    let globs = decode::ignore_globs(ignore)?;
    let walked = shell::walk(root, &globs);
    let files = decode::select_files(walked, root)?;
    let compiled = decode::compile_rules(rules)?;
    let rules = NonEmpty::new(compiled).expect("at least one rule");
    Ok(decide::plan(&rules, &files))
}

fn scan(root: &Path, rules: &[ParsedRule], ignore: &[String]) -> Result<Outcome, GritlintError> {
    let plan = plan(root, rules, ignore)?;
    let sources = shell::read_sources(root, &decide::sources_needed(&plan))?;
    decide::scan(&plan, &sources)
}

fn snapshot(root: &Path) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    collect(root, root, &mut entries);
    entries.sort();
    entries
}

fn collect(root: &Path, dir: &Path, entries: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).expect("read dir") {
        let path = entry.expect("entry").path();
        if path.is_dir() {
            collect(root, &path, entries);
        } else {
            entries.push((
                path.strip_prefix(root)
                    .expect("relative")
                    .to_string_lossy()
                    .replace('\\', "/"),
                fs::read_to_string(&path).expect("read file"),
            ));
        }
    }
}

#[test]
fn a_target_file_that_fails_to_parse_refuses_at_its_position_with_a_reason_code() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "tsconfig.json", "{\n  \"strict\": true,,\n}\n");

    let error = scan(temp.path(), &[rule("json", REACT_PAIR)], &[])
        .expect_err("a malformed target file is an instrument error");

    assert!(
        matches!(
            &error,
            GritlintError::TargetParse { file, line: 2, column: 17, .. } if file == "tsconfig.json"
        ),
        "{error:?}"
    );
    assert!(
        error
            .to_string()
            .starts_with("tsconfig.json:2:17: target-does-not-parse: "),
        "an agent reads the file, the position and the reason code from the line's head: {error}"
    );
}

#[test]
fn the_refusal_names_the_first_unparseable_file_in_plan_order() {
    let temp = tempfile::tempdir().expect("temp dir");
    for directory in ["a", "b"] {
        write(
            temp.path(),
            &format!("{directory}/tsconfig.json"),
            "{\"compilerOptions\": {",
        );
        write(
            temp.path(),
            &format!("{directory}/package.json"),
            "{\"vite\": \"1\"}",
        );
    }

    let error = scan(temp.path(), &[rule("join", SIBLING_JOIN)], &[])
        .expect_err("a malformed target file is an instrument error");

    assert!(
        error.to_string().contains("a/tsconfig.json"),
        "the first directory's file must be named, whatever the thread schedule: {error}"
    );
}

#[test]
fn an_unreadable_file_refuses_the_run_only_when_a_rule_evaluates_it() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "package.json", "{\"react\": \"18\"}\n");
    fs::write(temp.path().join("broken.ts"), [0xff, 0xfe]).expect("write the ts file");

    let outcome = scan(temp.path(), &[rule("react", REACT_PAIR)], &[])
        .expect("no json rule reads a typescript file");
    assert_eq!(outcome.findings().len(), 1, "{outcome:#?}");

    fs::write(temp.path().join("broken.json"), [0xff, 0xfe]).expect("write the json file");
    let error = scan(temp.path(), &[rule("react", REACT_PAIR)], &[])
        .expect_err("a json file the rule evaluates must be read");
    assert!(matches!(error, GritlintError::Io { .. }), "{error}");
    assert!(error.to_string().contains("broken.json"), "{error}");
}

#[test]
fn a_planned_file_missing_from_the_sources_refuses_the_scan() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "package.json", "{\"react\": \"18\"}\n");
    let plan = plan(temp.path(), &[rule("react", REACT_PAIR)], &[]).expect("plan");

    let error = decide::scan(&plan, &Sources::from_iter([]))
        .expect_err("a rule must never evaluate a file it was not given");

    assert!(
        matches!(error, GritlintError::SourceUnread { ref path } if path == "package.json"),
        "{error}"
    );
}

#[test]
fn a_rule_that_evaluates_zero_files_is_named_on_the_success_line() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(
        temp.path(),
        "packages/app/package.json",
        "{\"name\": \"app\"}",
    );

    let outcome = scan(temp.path(), &[rule("api", API_EXTRACTOR)], &[]).expect("a clean scan");

    assert!(outcome.findings().is_empty(), "{outcome:#?}");
    assert_eq!(decide::exit_code(outcome.verdict()), 0);
    assert_eq!(outcome.zero_file_rules(), vec!["local/api".to_owned()]);
    let human = encode::render_human(&outcome);
    assert!(human.contains("local/api: evaluated 0 file(s)"), "{human}");
    assert!(human.contains("zero-file rules: local/api"), "{human}");
}

#[test]
fn a_violation_carries_the_rule_path_line_and_description() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(
        temp.path(),
        "package.json",
        "{\n  \"name\": \"app\",\n  \"console\": \"debug\"\n}\n",
    );
    let rule = markdown_rule(
        "no-console",
        "Remove the debug console call.",
        "language json\n`\"console\": $_`\n",
    );

    let outcome = scan(temp.path(), std::slice::from_ref(&rule), &[]).expect("scan");

    assert_eq!(decide::exit_code(outcome.verdict()), 1);
    assert_eq!(outcome.findings().len(), 1, "{outcome:#?}");
    let finding = &outcome.findings()[0];
    assert_eq!(finding.rule_id().as_string(), "local/no-console");
    assert_eq!(finding.location().path().as_str(), "package.json");
    assert_eq!(
        finding.location().range(),
        Some(&Range {
            start: Position { line: 3, column: 3 },
            end: Position {
                line: 3,
                column: 21
            },
        })
    );
    assert_eq!(finding.message(), "Remove the debug console call.");
}

#[test]
fn a_tree_of_node_modules_and_gitignored_files_names_the_root() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), ".gitignore", "ignored.json\n");
    write(temp.path(), "node_modules/pkg.json", "{\"name\": \"dep\"}");
    write(temp.path(), "ignored.json", "{\"name\": \"ignored\"}");

    let error = scan(temp.path(), &[rule("json", REACT_PAIR)], &[])
        .expect_err("selecting no files is an instrument error");

    assert!(
        matches!(error, GritlintError::NoFilesSelected { .. }),
        "{error}"
    );
    assert!(error.to_string().contains(temp.path().to_str().unwrap()));
}

#[test]
fn no_enabled_pack_and_no_rule_path_is_an_instrument_error() {
    let error = decode::load_rules(
        &[],
        &BTreeMap::new(),
        Vec::new(),
        Path::new("gritlint.json"),
    )
    .expect_err("a run with no rules must not report a vacuous pass");

    assert!(
        matches!(error, GritlintError::NoRulesEnabled { .. }),
        "{error}"
    );
}

#[test]
fn json_output_is_the_versioned_envelope() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "a/package.json", "{\"react\": \"18\"}\n");
    write(temp.path(), "b/package.json", "{\"react\": \"19\"}\n");

    let outcome = scan(temp.path(), &[rule("react", REACT_PAIR)], &[]).expect("scan");
    let rendered = encode::render_json(&outcome).expect("render json");
    let parsed: serde_json::Value = serde_json::from_str(&rendered).expect("parseable json");

    let finding = |path: &str| {
        serde_json::json!({
            "ruleId": "local/react",
            "reasonCode": "test-reason",
            "message": "react",
            "nextAction": "Fix it.",
            "location": {
                "path": path,
                "range": {"start": {"line": 1, "column": 2}, "end": {"line": 1, "column": 15}},
            },
        })
    };
    assert_eq!(
        parsed,
        serde_json::json!({
            "schemaVersion": 1,
            "findings": [finding("a/package.json"), finding("b/package.json")],
            "summary": {
                "filesSelected": 2,
                "findings": 2,
                "rules": [{"ruleId": "local/react", "files": 2}],
                "zeroFileRules": [],
            },
        })
    );
}

#[test]
fn the_same_tree_renders_byte_identical_output() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "package.json", "{\"react\": \"18\"}\n");

    let first = scan(temp.path(), &[rule("react", REACT_PAIR)], &[]).expect("first scan");
    let second = scan(temp.path(), &[rule("react", REACT_PAIR)], &[]).expect("second scan");

    assert_eq!(encode::render_human(&first), encode::render_human(&second));
    assert_eq!(
        encode::render_json(&first).expect("first json"),
        encode::render_json(&second).expect("second json")
    );
}

#[test]
fn a_scan_writes_nothing_to_the_tree() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "package.json", "{\"react\": \"18\"}\n");
    write(temp.path(), "nested/deep.json", "{\"react\": \"19\"}\n");
    let before = snapshot(temp.path());

    scan(temp.path(), &[rule("react", REACT_PAIR)], &[]).expect("scan");

    assert_eq!(before, snapshot(temp.path()));
}

#[test]
fn a_multifile_rule_reports_one_finding_per_directory() {
    let temp = tempfile::tempdir().expect("temp dir");
    for directory in ["a", "b"] {
        write(
            temp.path(),
            &format!("{directory}/tsconfig.json"),
            "{\"compilerOptions\": {}}",
        );
        write(
            temp.path(),
            &format!("{directory}/package.json"),
            "{\"vite\": \"1\"}",
        );
    }

    let outcome = scan(temp.path(), &[rule("join", SIBLING_JOIN)], &[]).expect("scan");

    assert_eq!(outcome.findings().len(), 2, "{outcome:#?}");
    assert_eq!(
        outcome
            .findings()
            .iter()
            .map(|finding| finding.location().path().as_str().to_owned())
            .collect::<Vec<_>>(),
        ["a/package.json", "b/package.json"]
    );
    assert!(outcome.findings()[0].message().contains("(directory `a`)"));
    assert!(outcome.findings()[1].message().contains("(directory `b`)"));
    assert_eq!(outcome.counts()[0].files(), 4);
}

#[test]
fn a_multifile_rule_does_not_join_files_from_different_directories() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "a/tsconfig.json", "{\"compilerOptions\": {}}");
    write(temp.path(), "b/package.json", "{\"vite\": \"1\"}");

    let outcome = scan(temp.path(), &[rule("join", SIBLING_JOIN)], &[]).expect("scan");

    assert!(outcome.findings().is_empty(), "{outcome:#?}");
    assert_eq!(decide::exit_code(outcome.verdict()), 0);
}

#[test]
fn a_multifile_rule_only_receives_the_files_its_name_patterns_match() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "a/tsconfig.json", "{\"compilerOptions\": {}}");
    write(temp.path(), "a/package.json", "{\"vite\": \"1\"}");
    write(temp.path(), "a/other.json", "{\"unrelated\": true}");

    let outcome = scan(temp.path(), &[rule("join", SIBLING_JOIN)], &[]).expect("scan");

    assert_eq!(outcome.counts()[0].files(), 2, "{outcome:#?}");
}

#[test]
fn a_single_file_rule_reports_each_match_at_its_file() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "a/package.json", "{\"react\": \"18\"}\n");
    write(temp.path(), "b/package.json", "{\"react\": \"19\"}\n");

    let outcome = scan(temp.path(), &[rule("react", REACT_PAIR)], &[]).expect("scan");

    assert_eq!(outcome.findings().len(), 2, "{outcome:#?}");
    assert_eq!(
        outcome
            .findings()
            .iter()
            .map(|finding| finding.location().path().as_str().to_owned())
            .collect::<Vec<_>>(),
        ["a/package.json", "b/package.json"]
    );
    assert_eq!(outcome.counts()[0].files(), 2);
}

#[test]
fn a_non_utf8_file_no_rule_can_target_is_never_read() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "package.json", "{\"react\": \"18\"}\n");
    fs::write(temp.path().join("fixture.tar.gz"), [0x1f, 0x8b, 0xff, 0xfe])
        .expect("write the archive");

    let outcome = scan(temp.path(), &[rule("react", REACT_PAIR)], &[]).expect("scan");

    assert_eq!(outcome.findings().len(), 1, "{outcome:#?}");
    assert_eq!(
        outcome.findings()[0].location().path().as_str(),
        "package.json"
    );
}

#[test]
fn ignore_globs_drop_files_from_every_rule_batch() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "generated/skip.json", "{\"react\": \"18\"}");
    write(temp.path(), "real.json", "{\"react\": \"19\"}");

    let outcome = scan(
        temp.path(),
        &[rule("react", REACT_PAIR)],
        &["generated/**".to_owned()],
    )
    .expect("scan");

    assert_eq!(outcome.counts()[0].files(), 1, "{outcome:#?}");
    assert_eq!(outcome.findings().len(), 1);
    assert_eq!(
        outcome.findings()[0].location().path().as_str(),
        "real.json"
    );
}

const TSDOWN_PRESENCE_JOIN: &str = r#"language json
multifile {
  bubble($dir) file($name, $body) where {
    $name <: r"(.*/)tsdown\.config\.ts"($dir)
  },
  bubble($dir) file($name, $body) where {
    $name <: r"(.*/)package\.json"($dir),
    $body <: contains `"react": $_`
  }
}
"#;

const PACKAGE_ONLY_JOIN: &str = r#"language json
multifile {
  bubble($dir) file($name, $body) where {
    $name <: r"(.*/)package\.json"($dir),
    $body <: contains `"react": $_`
  }
}
"#;

#[test]
fn a_multifile_rule_joins_a_name_matched_file_of_another_language() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "pkg/package.json", "{\"react\": \"18\"}");
    write(
        temp.path(),
        "pkg/tsdown.config.ts",
        "export default defineConfig({ entry: './src/index.ts' })\n",
    );

    let outcome = scan(temp.path(), &[rule("join", TSDOWN_PRESENCE_JOIN)], &[])
        .expect("a name-only participant must not be parsed as the rule's language");

    assert_eq!(outcome.findings().len(), 1, "{outcome:#?}");
    assert_eq!(
        outcome.findings()[0].location().path().as_str(),
        "pkg/package.json"
    );
    assert_eq!(outcome.counts()[0].files(), 2, "{outcome:#?}");
}

#[test]
fn a_multifile_rule_pulls_in_only_the_other_language_files_its_names_match() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "pkg/package.json", "{\"react\": \"18\"}");
    write(temp.path(), "pkg/tsdown.config.ts", "export default {}\n");

    let outcome = scan(temp.path(), &[rule("join", PACKAGE_ONLY_JOIN)], &[]).expect("scan");

    assert_eq!(outcome.findings().len(), 1, "{outcome:#?}");
    assert_eq!(outcome.counts()[0].files(), 1, "{outcome:#?}");
}

fn locations(outcome: &Outcome) -> Vec<String> {
    outcome
        .findings()
        .iter()
        .map(decide::expected_location_line)
        .collect()
}

#[test]
fn a_multifile_finding_goes_to_the_first_matched_file_its_locator_matches() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "a/package.json", "{\"vite\": \"1\"}\n");
    write(
        temp.path(),
        "a/tsconfig.json",
        "{\n  \"compilerOptions\": {}\n}\n",
    );
    let rule = located_rule("join", SIBLING_JOIN, r#"`"compilerOptions": $_`"#);

    let outcome = scan(temp.path(), &[rule], &[]).expect("scan");

    assert_eq!(
        locations(&outcome),
        ["local/join a/tsconfig.json:2:3-2:24"],
        "the locator matches only the second matched file, at line 2"
    );
}

#[test]
fn a_multifile_locator_matching_several_nodes_takes_the_earliest_in_the_first_file() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(
        temp.path(),
        "a/package.json",
        "{\n  \"vite\": \"1\",\n  \"probe\": 1,\n  \"probe\": 2\n}\n",
    );
    write(
        temp.path(),
        "a/tsconfig.json",
        "{\n  \"probe\": 0,\n  \"compilerOptions\": {}\n}\n",
    );
    let rule = located_rule("join", SIBLING_JOIN, r#"`"probe": $_`"#);

    let outcome = scan(temp.path(), &[rule], &[]).expect("scan");

    assert_eq!(
        locations(&outcome),
        ["local/join a/package.json:3:3-3:13"],
        "both files match the locator: the first path wins, and its earliest node"
    );
}

#[test]
fn a_multifile_locator_that_matches_nothing_leaves_the_first_matched_file_alone() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "a/package.json", "{\"vite\": \"1\"}\n");
    write(
        temp.path(),
        "a/tsconfig.json",
        "{\"compilerOptions\": {}}\n",
    );
    let rule = located_rule("join", SIBLING_JOIN, r#"`"absent": $_`"#);

    let outcome = scan(temp.path(), &[rule], &[]).expect("scan");

    assert_eq!(locations(&outcome), ["local/join a/package.json"]);
    assert_eq!(outcome.findings()[0].location().range(), None);
}

#[test]
fn a_single_file_rule_reports_every_node_it_matches_in_a_file() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(
        temp.path(),
        "package.json",
        "{\n  \"react\": \"18\",\n  \"peer\": {\"react\": \"19\"}\n}\n",
    );

    let outcome = scan(temp.path(), &[rule("react", REACT_PAIR)], &[]).expect("scan");

    assert_eq!(
        locations(&outcome),
        [
            "local/react package.json:2:3-2:16",
            "local/react package.json:3:12-3:25",
        ]
    );
}

#[test]
fn a_single_file_match_without_a_range_is_reported_at_its_file_alone() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "package.json", "{\"react\": \"18\"}\n");
    let whole_file = "language json\nfile($name, $body) where { $name <: r\".*package\\.json\" }\n";

    let outcome = scan(temp.path(), &[rule("whole-file", whole_file)], &[]).expect("scan");

    assert_eq!(locations(&outcome), ["local/whole-file package.json"]);
    assert_eq!(decide::exit_code(outcome.verdict()), 1);
}
