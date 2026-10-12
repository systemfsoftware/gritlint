#![allow(clippy::expect_used, clippy::disallowed_methods)]

use gritlint_core::decide;
use gritlint_core::decode;
use gritlint_core::domain::{
    CompiledRule, EmbeddedFile, Language, NonEmpty, Pack, ParsedRule, RelPath,
};
use gritlint_core::engine::{self, CompileRequest, InputFile, MatchRange};
use gritlint_core::export;
use gritlint_core::shell;
use gritlint_core::workspace::{self, Workspace, WorkspaceInputs};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const STUB: &str = "predicate register_diagnostic($span, $message, $severity) { log(message = $message), log(message = $span), log(message = $severity) }\n";

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/export")
}

fn committed(relative: &str) -> String {
    fs::read_to_string(fixtures().join(relative)).expect("read a committed fixture")
}

fn packs_in(dir: &Path) -> Vec<Pack> {
    let tree = shell::read_tree(dir).expect("read the packs");
    let files: Vec<EmbeddedFile> = tree
        .files()
        .iter()
        .map(|file| {
            EmbeddedFile::new(
                RelPath::from_root(tree.root(), file.path())
                    .expect("a file below the packs root")
                    .as_str()
                    .to_owned(),
                file.contents().to_owned(),
            )
        })
        .collect();
    decode::embedded_packs(&files).expect("decode the packs")
}

fn load(config: &str, packs: &Path, root: &str) -> Workspace {
    let inputs = WorkspaceInputs {
        config: Some(fixtures().join(config)),
        rules: Vec::new(),
        root: fixtures().join(root),
    };
    workspace::load(&inputs, &packs_in(packs)).expect("load the workspace")
}

fn fixture_workspace() -> Workspace {
    load("gritlint.json", &fixtures().join("packs"), "tree")
}

fn rules(workspace: &Workspace) -> Vec<&ParsedRule> {
    workspace.rules().iter().map(CompiledRule::rule).collect()
}

fn exported(rules: &[&ParsedRule], id: &str) -> Result<String, (&'static str, String)> {
    export::find(rules, id)
        .and_then(|rule| export::biome(rule, rules))
        .map_err(|refusal| (refusal.reason_code(), refusal.to_string()))
}

#[test]
fn listing_names_each_enabled_rule_and_whether_it_exports() {
    let workspace = fixture_workspace();

    assert_eq!(
        export::listing(&rules(&workspace)),
        committed("expected/listing.txt")
    );
}

#[test]
fn each_exportable_rule_renders_its_committed_plugin() {
    let workspace = fixture_workspace();
    let rules = rules(&workspace);

    for name in [
        "forbidden-license",
        "license_value",
        "no-console-log",
        "no-important",
        "own-helper",
    ] {
        assert_eq!(
            exported(&rules, &format!("export-fixtures/{name}")),
            Ok(committed(&format!("expected/{name}.grit"))),
            "{name}"
        );
    }
}

#[test]
fn each_refused_rule_reports_its_own_reason_code() {
    let workspace = fixture_workspace();
    let rules = rules(&workspace);

    for (name, code) in [
        ("multifile-join", "export-multifile"),
        ("commented-multifile", "export-multifile"),
        ("yaml-name", "export-language"),
        ("flow-console", "export-language"),
        ("js-function", "export-construct"),
        ("rewrite-license", "export-rewrite"),
        ("calls-sibling", "export-library"),
        ("calls-sibling-helper", "export-library"),
        ("nope", "export-unknown-rule"),
    ] {
        let refused = exported(&rules, &format!("export-fixtures/{name}"));
        assert_eq!(refused.map_err(|(code, _)| code), Err(code), "{name}");
    }
}

#[test]
fn a_refusal_names_the_rule_its_reason_and_a_next_action() {
    let workspace = fixture_workspace();
    let rules = rules(&workspace);

    assert_eq!(
        exported(&rules, "export-fixtures/calls-sibling").map_err(|(_, text)| text),
        Err("export-fixtures/calls-sibling: export-library: the rule calls `license_value`, which another rule of its pack defines, and a plugin cannot reach it. Next action: inline the called rule's pattern into this rule, or keep this rule in `gritlint check`.".to_owned())
    );
    assert_eq!(
        exported(&rules, "export-fixtures/calls-sibling-helper").map_err(|(_, text)| text),
        Err("export-fixtures/calls-sibling-helper: export-library: the rule calls `unlicensed_text`, which another rule of its pack defines, and a plugin cannot reach it. Next action: inline the called rule's pattern into this rule, or keep this rule in `gritlint check`.".to_owned())
    );
    assert_eq!(
        exported(&rules, "export-fixtures/nope").map_err(|(_, text)| text),
        Err("export-fixtures/nope: export-unknown-rule: no enabled rule has this id. Next action: run `gritlint export biome` without `--rule` to list the enabled rule ids.".to_owned())
    );
}

#[test]
fn every_bundled_rule_lists_as_a_multifile_refusal() {
    let packs = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs");
    let workspace = load("bundled/gritlint.json", &packs, "tree");

    assert_eq!(
        export::listing(&rules(&workspace)),
        committed("expected/bundled-listing.txt")
    );
}

fn span(range: &MatchRange) -> (u32, u32, u32, u32) {
    (
        range.start_line,
        range.start_column,
        range.end_line,
        range.end_column,
    )
}

#[test]
fn the_committed_plugin_registers_one_diagnostic_per_finding_in_gritlints_engine() {
    let plugin = committed("expected/forbidden-license.grit");
    let (language_line, rest) = plugin.split_once('\n').expect("a language line");
    let body = format!("{language_line}\n{STUB}{rest}");
    let request = CompileRequest {
        language: Language::of_body(&body).expect("a language").engine(),
        body,
        library: Vec::new(),
        parameters: BTreeMap::new(),
    };
    let outcome = engine::compile(&request)
        .expect("the plugin compiles in gritlint's engine")
        .execute(vec![InputFile {
            path: "package.json".to_owned(),
            contents: committed("tree/package.json"),
        }]);

    let ranges: Vec<_> = outcome
        .matches
        .iter()
        .flat_map(|matched| matched.ranges.iter().map(span))
        .collect();
    assert_eq!(ranges, vec![(2, 3, 2, 26), (3, 15, 3, 40)]);

    let mut logged: Vec<&str> = outcome
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.message.as_str())
        .collect();
    logged.sort_unstable();
    assert_eq!(
        logged,
        vec![
            "\"license\":   \"UNLICENSED\"\n",
            "\"license\": \"UNLICENSED\"\n",
            "A package does not use a \"forbidden\" license.\\nThe pack names the license it forbids.\n",
            "A package does not use a \"forbidden\" license.\\nThe pack names the license it forbids.\n",
            "error\n",
            "error\n",
        ]
    );
}

#[test]
fn check_reports_the_same_two_ranges_the_plugin_matches() {
    let workspace = fixture_workspace();
    let rule = workspace
        .rules()
        .iter()
        .find(|compiled| compiled.rule().id().as_string() == "export-fixtures/forbidden-license")
        .expect("the rule is enabled")
        .clone();
    let files = workspace.select().expect("select the tree");
    let plan = decide::plan(&NonEmpty::from_head(rule, Vec::new()), &files);
    let sources =
        shell::read_sources(workspace.root(), &decide::sources_needed(&plan)).expect("read");
    let outcome = decide::scan(&plan, &sources).expect("scan");

    let found: Vec<String> = outcome
        .findings()
        .iter()
        .map(|finding| finding.location().to_string())
        .collect();
    assert_eq!(found, vec!["package.json:2:3", "package.json:3:15"]);
    let ends: Vec<_> = outcome
        .findings()
        .iter()
        .filter_map(|finding| finding.location().range())
        .map(|range| (range.end.line, range.end.column))
        .collect();
    assert_eq!(ends, vec![(2, 26), (3, 40)]);
}
