#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use gritlint_core::domain::{
    Finding, Guidance, Location, Outcome, ReasonCode, RelPath, RuleCount, RuleId, RuleName,
};
use gritlint_core::encode;
use std::sync::Arc;

const RULE: &str = "concise-test";
const REASON: &str = "generated";

fn finding(path: &str, message: &str) -> Finding {
    Finding::new(
        RuleId::local(RuleName::new(RULE)),
        Location::new(RelPath::new(path).expect("a normalized path"), None),
        message.to_owned(),
        Arc::new(Guidance::new(
            ReasonCode::new(REASON).expect("a kebab-case reason code"),
            "Fix it.".to_owned(),
        )),
    )
}

fn single(path: &str, message: &str) -> Outcome {
    Outcome::new(vec![finding(path, message)], Vec::new(), 3)
}

#[test]
fn a_concise_line_normalises_whitespace_and_renders_whole_under_its_byte_budget() {
    let outcome = single("a/package.json", "Fix  the\tthing now");
    assert_eq!(
        encode::render_concise(&outcome),
        "a/package.json local/concise-test generated Fix the thing now\n1 finding(s) in 3 file(s) selected\n"
    );
}

#[test]
fn a_concise_line_cuts_the_message_at_the_byte_budget_with_one_trailing_ellipsis() {
    let outcome = single("a/package.json", &"a".repeat(300));
    let expected = format!(
        "a/package.json local/concise-test generated {}…\n1 finding(s) in 3 file(s) selected\n",
        "a".repeat(153)
    );
    assert_eq!(encode::render_concise(&outcome), expected);
}

/// KTD24 cuts the message first, then the start of the path. At `fixed` >= 196 the message is cut
/// to zero bytes and the line takes the long-path branch, the same branch every long-path line
/// takes, so this pins the stated contract (plan ruling Q2).
#[test]
fn a_concise_line_at_the_message_cutoff_drops_the_message() {
    let path = format!("pkg/{}.json", "d".repeat(158));
    let outcome = single(&path, &"a".repeat(300));
    let expected =
        format!("{path} local/concise-test generated\n1 finding(s) in 3 file(s) selected\n");
    assert_eq!(encode::render_concise(&outcome), expected);
}

#[test]
fn a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name() {
    let path = format!("d/{}/package.json", "d".repeat(244));
    let tail = format!("{}/package.json", "d".repeat(155));
    let outcome = single(&path, &"a".repeat(300));
    let expected =
        format!("…{tail} local/concise-test generated\n1 finding(s) in 3 file(s) selected\n");
    assert_eq!(encode::render_concise(&outcome), expected);
}

#[test]
fn a_concise_line_never_splits_a_multibyte_character() {
    let message = format!("{}é{}", "a".repeat(152), "z".repeat(50));
    let outcome = single("a/package.json", &message);
    let expected = format!(
        "a/package.json local/concise-test generated {}…\n1 finding(s) in 3 file(s) selected\n",
        "a".repeat(152)
    );
    assert_eq!(encode::render_concise(&outcome), expected);

    let path = format!("pkg/{}é{}", "d".repeat(77), "d".repeat(167));
    let tail = "d".repeat(167);
    let outcome = single(&path, &"a".repeat(300));
    let expected =
        format!("…{tail} local/concise-test generated\n1 finding(s) in 3 file(s) selected\n");
    assert_eq!(encode::render_concise(&outcome), expected);
}

#[test]
fn a_github_summary_row_escapes_pipes_backslashes_and_newlines() {
    let outcome = Outcome::new(
        vec![finding("a/|weird|.json", "Fix the\n|table| \\ row")],
        vec![RuleCount::new(RuleId::local(RuleName::new(RULE)), 2)],
        3,
    );
    let expected = concat!(
        "### gritlint\n\n1 finding(s) in 3 file(s) selected\n",
        "\n| Location | Rule | Reason | Message | Next action |\n| --- | --- | --- | --- | --- |\n",
        "| a/\\|weird\\|.json | local/concise-test | generated | Fix the \\|table\\| \\\\ row | Fix it. |\n",
        "\n| Rule | Files evaluated |\n| --- | --- |\n",
        "| local/concise-test | 2 |\n",
    );
    assert_eq!(encode::render_github_summary(&outcome), expected);
}

#[test]
fn the_report_schema_is_the_committed_envelope_schema() {
    let rendered = format!(
        "{}\n",
        serde_json::to_string_pretty(&encode::report_schema()).expect("the schema serializes")
    );
    assert_eq!(
        rendered,
        include_str!("../../../npm/gritlint/report_schema.json")
    );
}
