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
use gritlint_core::domain::{CaseKind, FixtureOutcome};
use gritlint_core::encode;
use gritlint_core::shell;
use std::fs;
use std::path::Path;

const NO_CONSOLE: &str = "---\nreasonCode: debug-console\nnextAction: Remove the console key.\n---\nlanguage json\n`\"console\": $_`\n";
const CONDITION_RULE: &str =
    "---\nreasonCode: condition-found\nnextAction: Remove it.\n---\nlanguage json\ncondition()\n";
const BASIC_EXPECTED: &str = "pack-one/no-console package.json:1:2-1:20\n";

const LOCATED_JOIN: &str = r#"---
reasonCode: missing-provenance
nextAction: Set publishConfig.provenance to true.
locate: `"provenance": $value` where { $value <: not `true` }
---
language json
multifile {
  file($name, $body) where {
    $name <: r".*package\.json",
    $body <: not contains `"provenance": true`
  }
}
"#;

fn write(base: &Path, path: &str, contents: &str) {
    let target = base.join(path);
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent).expect("create the parent directory");
    }
    fs::write(target, contents).expect("write the file");
}

fn write_pack(base: &Path, name: &str, required: &[&str], rule_file: &str, rule_body: &str) {
    let required: Vec<String> = required
        .iter()
        .map(|param| format!("\"{param}\""))
        .collect();
    write(
        base,
        &format!("{name}/pack.json"),
        &format!("{{\"requiredParams\": [{}]}}", required.join(", ")),
    );
    write(base, &format!("{name}/rules/{rule_file}"), rule_body);
}

fn run_fixtures(dir: &Path) -> Result<FixtureOutcome, GritlintError> {
    let tree = shell::read_tree(dir)?;
    let layout = decode::fixture_layout(&tree)?;
    decide::run_fixtures(&layout)
}

#[test]
fn a_pack_with_a_bad_and_a_good_case_passes() {
    let temp = tempfile::tempdir().expect("temp dir");
    write_pack(temp.path(), "pack-one", &[], "no-console.grit", NO_CONSOLE);
    write(
        temp.path(),
        "pack-one/fixtures/no-console/bad/basic/package.json",
        "{\"console\": \"debug\"}",
    );
    write(
        temp.path(),
        "pack-one/fixtures/no-console/bad/basic/expected-locations.txt",
        BASIC_EXPECTED,
    );
    write(
        temp.path(),
        "pack-one/fixtures/no-console/good/basic/package.json",
        "{\"name\": \"app\"}",
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    assert!(outcome.passed(), "{outcome:#?}");
    assert_eq!(outcome.results().len(), 2);
    assert_eq!(
        encode::render_fixtures(&outcome),
        concat!(
            "ok bad pack-one/no-console fixtures/no-console/bad/basic\n",
            "ok good pack-one/no-console fixtures/no-console/good/basic\n",
            "2 case(s) passed\n"
        )
    );
}

#[test]
fn a_bad_case_that_yields_no_finding_names_the_rule_and_the_case() {
    let temp = tempfile::tempdir().expect("temp dir");
    write_pack(temp.path(), "pack-one", &[], "no-console.grit", NO_CONSOLE);
    write(
        temp.path(),
        "pack-one/fixtures/no-console/bad/basic/package.json",
        "{\"name\": \"app\"}",
    );
    write(
        temp.path(),
        "pack-one/fixtures/no-console/good/basic/package.json",
        "{\"name\": \"app\"}",
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    assert!(!outcome.passed());
    let failures = outcome.failures();
    assert_eq!(failures.len(), 1, "{outcome:#?}");
    assert_eq!(failures[0].rule_id().as_string(), "pack-one/no-console");
    assert_eq!(failures[0].case().as_str(), "fixtures/no-console/bad/basic");
    assert_eq!(failures[0].kind(), CaseKind::Bad);
    assert!(
        failures[0].detail().contains("no finding"),
        "{}",
        failures[0].detail()
    );
    assert_eq!(
        encode::render_fixtures(&outcome),
        concat!(
            "FAIL bad pack-one/no-console fixtures/no-console/bad/basic: ",
            "the known-bad case produced no finding\n",
            "ok good pack-one/no-console fixtures/no-console/good/basic\n",
            "1 case(s) failed\n"
        )
    );
}

#[test]
fn a_bad_case_without_expected_locations_fails_listing_what_it_reports() {
    let temp = tempfile::tempdir().expect("temp dir");
    write_pack(temp.path(), "pack-one", &[], "no-console.grit", NO_CONSOLE);
    write(
        temp.path(),
        "pack-one/fixtures/no-console/bad/basic/package.json",
        "{\"console\": \"debug\"}",
    );
    write(
        temp.path(),
        "pack-one/fixtures/no-console/good/basic/package.json",
        "{\"name\": \"app\"}",
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    let failures = outcome.failures();
    assert_eq!(failures.len(), 1, "{outcome:#?}");
    assert_eq!(failures[0].case().as_str(), "fixtures/no-console/bad/basic");
    assert!(
        failures[0].detail().contains(BASIC_EXPECTED.trim_end()),
        "{}",
        failures[0].detail()
    );
}

#[test]
fn a_good_case_that_yields_a_finding_names_the_finding() {
    let temp = tempfile::tempdir().expect("temp dir");
    write_pack(temp.path(), "pack-one", &[], "no-console.grit", NO_CONSOLE);
    write(
        temp.path(),
        "pack-one/fixtures/no-console/bad/basic/package.json",
        "{\"console\": \"debug\"}",
    );
    write(
        temp.path(),
        "pack-one/fixtures/no-console/bad/basic/expected-locations.txt",
        BASIC_EXPECTED,
    );
    write(
        temp.path(),
        "pack-one/fixtures/no-console/good/basic/package.json",
        "{\"console\": \"debug\"}",
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    let failures = outcome.failures();
    assert_eq!(failures.len(), 1, "{outcome:#?}");
    assert_eq!(failures[0].kind(), CaseKind::Good);
    assert!(
        failures[0].detail().contains("package.json")
            && failures[0].detail().contains("no-console"),
        "{}",
        failures[0].detail()
    );
}

#[test]
fn a_rule_with_only_good_cases_fails_as_untested() {
    let temp = tempfile::tempdir().expect("temp dir");
    write_pack(temp.path(), "pack-one", &[], "no-console.grit", NO_CONSOLE);
    write(
        temp.path(),
        "pack-one/fixtures/no-console/good/basic/package.json",
        "{\"name\": \"app\"}",
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    assert!(!outcome.passed());
    let failures = outcome.failures();
    assert_eq!(failures.len(), 1, "{outcome:#?}");
    assert_eq!(failures[0].kind(), CaseKind::Bad);
    assert!(
        failures[0].detail().contains("no known-bad case"),
        "{}",
        failures[0].detail()
    );
    assert_eq!(failures[0].case().as_str(), "fixtures/no-console/bad");
    let rendered = encode::render_fixtures(&outcome);
    assert!(
        rendered
            .contains("FAIL bad pack-one/no-console fixtures/no-console/bad: no known-bad case"),
        "{rendered}"
    );
}

#[test]
fn a_consumer_rules_directory_follows_the_same_contract() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "no-console.grit", NO_CONSOLE);
    write(
        temp.path(),
        "fixtures/no-console/good/basic/package.json",
        "{\"name\": \"app\"}",
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    let failures = outcome.failures();
    assert_eq!(failures.len(), 1, "{outcome:#?}");
    assert_eq!(failures[0].rule_id().as_string(), "local/no-console");
    assert!(
        failures[0].detail().contains("no known-bad case"),
        "{}",
        failures[0].detail()
    );
}

#[test]
fn a_consumer_layout_ignores_a_non_rule_file_at_the_root() {
    let temp = tempfile::tempdir().expect("temp dir");
    write(temp.path(), "no-console.grit", NO_CONSOLE);
    write(temp.path(), "notes.txt", "these are not GritQL\n");
    write(
        temp.path(),
        "fixtures/no-console/good/basic/package.json",
        "{\"name\": \"app\"}",
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    assert!(!outcome.results().is_empty(), "{outcome:#?}");
    for result in outcome.results() {
        assert_eq!(
            result.rule_id().as_string(),
            "local/no-console",
            "{outcome:#?}"
        );
    }
}

#[test]
fn a_case_ignores_node_modules_and_dot_directories() {
    let temp = tempfile::tempdir().expect("temp dir");
    write_pack(temp.path(), "pack-one", &[], "no-console.grit", NO_CONSOLE);
    write(
        temp.path(),
        "pack-one/fixtures/no-console/bad/basic/package.json",
        "{\"console\": \"debug\"}",
    );
    write(
        temp.path(),
        "pack-one/fixtures/no-console/bad/basic/expected-locations.txt",
        BASIC_EXPECTED,
    );
    write(
        temp.path(),
        "pack-one/fixtures/no-console/good/basic/package.json",
        "{\"name\": \"app\"}",
    );
    for case in ["bad", "good"] {
        write(
            temp.path(),
            &format!("pack-one/fixtures/no-console/{case}/basic/node_modules/dep.json"),
            "{\"console\": \"debug\"}",
        );
        write(
            temp.path(),
            &format!("pack-one/fixtures/no-console/{case}/basic/.cache/scratch.json"),
            "{\"console\": \"debug\"}",
        );
    }

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    assert!(outcome.passed(), "{outcome:#?}");
}

#[test]
fn a_case_config_supplies_pack_parameters() {
    let temp = tempfile::tempdir().expect("temp dir");
    write_pack(
        temp.path(),
        "pack-one",
        &["condition"],
        "react.grit",
        CONDITION_RULE,
    );
    let config = "{\"packs\": {\"pack-one\": {\"condition\": \"`\\\"react\\\": $_`\"}}}";
    write(
        temp.path(),
        "pack-one/fixtures/react/bad/basic/package.json",
        "{\"react\": \"18\"}",
    );
    write(
        temp.path(),
        "pack-one/fixtures/react/bad/basic/gritlint.json",
        config,
    );
    write(
        temp.path(),
        "pack-one/fixtures/react/bad/basic/expected-locations.txt",
        "pack-one/react package.json:1:2-1:15\n",
    );
    write(
        temp.path(),
        "pack-one/fixtures/react/good/basic/package.json",
        "{\"name\": \"app\"}",
    );
    write(
        temp.path(),
        "pack-one/fixtures/react/good/basic/gritlint.json",
        config,
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    assert!(outcome.passed(), "{outcome:#?}");
}

fn located_pack(base: &Path, bad_package: &str, expected: &str) {
    write_pack(base, "pack-one", &[], "provenance.grit", LOCATED_JOIN);
    write(
        base,
        "pack-one/fixtures/provenance/bad/basic/package.json",
        bad_package,
    );
    write(
        base,
        "pack-one/fixtures/provenance/bad/basic/expected-locations.txt",
        expected,
    );
    write(
        base,
        "pack-one/fixtures/provenance/good/basic/package.json",
        "{\"publishConfig\": {\"provenance\": true}}",
    );
}

#[test]
fn a_bad_case_reported_at_another_node_than_it_expects_names_the_rule_and_the_case() {
    let temp = tempfile::tempdir().expect("temp dir");
    located_pack(
        temp.path(),
        "{\n  \"provenance\": \"no\",\n  \"publishConfig\": {\"provenance\": false}\n}\n",
        "pack-one/provenance package.json:3:21-3:40\n",
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    let failures = outcome.failures();
    assert_eq!(failures.len(), 1, "{outcome:#?}");
    assert_eq!(failures[0].rule_id().as_string(), "pack-one/provenance");
    assert_eq!(failures[0].case().as_str(), "fixtures/provenance/bad/basic");
    assert!(
        failures[0]
            .detail()
            .contains("reported pack-one/provenance package.json:2:3-2:21"),
        "{}",
        failures[0].detail()
    );
}

#[test]
fn a_bad_case_reported_where_it_expects_passes() {
    let temp = tempfile::tempdir().expect("temp dir");
    located_pack(
        temp.path(),
        "{\n  \"name\": \"pkg\",\n  \"publishConfig\": {\"provenance\": false}\n}\n",
        "pack-one/provenance package.json:3:21-3:40\n",
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    assert!(outcome.passed(), "{outcome:#?}");
}

#[test]
fn a_rule_with_a_locator_but_no_bad_case_asserting_a_range_fails() {
    let temp = tempfile::tempdir().expect("temp dir");
    located_pack(
        temp.path(),
        "{\"name\": \"pkg\"}",
        "pack-one/provenance package.json\n",
    );

    let outcome = run_fixtures(temp.path()).expect("run the fixtures");

    let failures = outcome.failures();
    assert_eq!(failures.len(), 1, "{outcome:#?}");
    assert_eq!(failures[0].rule_id().as_string(), "pack-one/provenance");
    assert!(
        failures[0].detail().contains("declares `locate`"),
        "{}",
        failures[0].detail()
    );
}
