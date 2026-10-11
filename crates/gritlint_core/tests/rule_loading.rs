#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

use gritlint_core::GritlintError;
use gritlint_core::decode;
use gritlint_core::domain::{CompiledRule, NonEmpty, Pack, PackName, ParamName, RuleSource};
use gritlint_core::error::FrontmatterProblem;
use gritlint_core::shell;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const FRONTMATTER: &str = "---\nreasonCode: test-reason\nnextAction: Fix it.\n---\n";

const REACT_RULE: &str = r"---
reasonCode: unpinned-react
nextAction: Pin the react dependency.
---

# React dependency

The `react` dependency must be pinned.

```grit
language json
condition()
```
";

const PLAIN_RULE: &str = r#"---
reasonCode: test-reason
nextAction: Fix it.
---
language json
`"react": $_`
"#;

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create the parent directory");
    }
    fs::write(path, contents).expect("write the file");
}

fn make_pack(root: &Path, name: &str, required: &[&str], rules: &[(&str, &str)]) -> PathBuf {
    let dir = root.join(name);
    let required: Vec<String> = required
        .iter()
        .map(|param| format!("\"{param}\""))
        .collect();
    write(
        &dir.join("pack.json"),
        &format!("{{\"requiredParams\": [{}]}}", required.join(", ")),
    );
    for (file, contents) in rules {
        write(&dir.join("rules").join(file), contents);
    }
    dir
}

fn pack_of(dir: &Path) -> Pack {
    let tree = shell::read_tree(dir).expect("read the pack tree");
    decode::pack_from_tree(&tree, dir).expect("decode the pack")
}

fn enabled(name: &str, params: &[(&str, &str)]) -> BTreeMap<PackName, BTreeMap<ParamName, String>> {
    BTreeMap::from([(
        PackName::new(name),
        params
            .iter()
            .map(|(key, value)| {
                (
                    ParamName::new(key).expect("a valid parameter name"),
                    (*value).to_owned(),
                )
            })
            .collect(),
    )])
}

fn sources(paths: &[PathBuf]) -> Vec<RuleSource> {
    shell::read_rule_sources(paths).expect("read the consumer sources")
}

fn ids(rules: &NonEmpty<CompiledRule>) -> Vec<String> {
    rules
        .iter()
        .map(|rule| rule.rule().id().as_string())
        .collect()
}

fn error_of(result: Result<NonEmpty<CompiledRule>, GritlintError>) -> GritlintError {
    result.expect_err("the load must fail")
}

#[test]
fn an_enabled_pack_loads_its_rules_under_its_namespace() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = make_pack(
        temp.path(),
        "source-resolution",
        &["condition"],
        &[("react.md", REACT_RULE)],
    );

    let rules = decode::load_rules(
        &[pack_of(&dir)],
        &enabled("source-resolution", &[("condition", "`\"react\": $_`")]),
        Vec::new(),
        Path::new("gritlint.json"),
    )
    .expect("load the pack");

    assert_eq!(ids(&rules), ["source-resolution/react"]);
    assert_eq!(
        rules.first().rule().message(),
        "The `react` dependency must be pinned."
    );
}

#[test]
fn with_no_pack_enabled_only_the_local_rule_loads() {
    let temp = tempfile::tempdir().expect("temp dir");
    let consumer = temp.path().join("local-rule.grit");
    write(&consumer, PLAIN_RULE);

    let rules = decode::load_rules(
        &[],
        &BTreeMap::new(),
        sources(std::slice::from_ref(&consumer)),
        Path::new("gritlint.json"),
    )
    .expect("load the consumer rule");

    assert_eq!(ids(&rules), ["local/local-rule"]);
    assert_eq!(rules.first().rule().message(), "local-rule");
}

#[test]
fn a_consumer_rule_named_like_a_bundled_rule_names_both_sources() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = make_pack(temp.path(), "pack-one", &[], &[("react.grit", PLAIN_RULE)]);
    let consumer = temp.path().join("react.grit");
    write(&consumer, PLAIN_RULE);

    let error = error_of(decode::load_rules(
        &[pack_of(&dir)],
        &enabled("pack-one", &[]),
        sources(std::slice::from_ref(&consumer)),
        Path::new("gritlint.json"),
    ));

    assert!(
        matches!(error, GritlintError::RuleCollision { .. }),
        "{error}"
    );
    let message = error.to_string();
    assert!(
        message.contains("pack-one/rules/react.grit") && message.contains("react.grit"),
        "both sources must be named: {message}"
    );
}

#[test]
fn a_markdown_rule_without_a_grit_block_names_the_file() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = make_pack(
        temp.path(),
        "pack-one",
        &[],
        &[("bad.md", &format!("{FRONTMATTER}# Title\n\nNo block.\n"))],
    );

    let error = error_of(decode::load_rules(
        &[pack_of(&dir)],
        &enabled("pack-one", &[]),
        Vec::new(),
        Path::new("gritlint.json"),
    ));

    assert!(
        matches!(error, GritlintError::RuleNoGritBlock { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("bad.md"), "{error}");
}

#[test]
fn an_unknown_pack_lists_the_known_packs() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = make_pack(temp.path(), "pack-one", &[], &[("react.grit", PLAIN_RULE)]);

    let error = error_of(decode::load_rules(
        &[pack_of(&dir)],
        &enabled("no-such-pack", &[]),
        Vec::new(),
        Path::new("gritlint.json"),
    ));

    assert!(
        matches!(error, GritlintError::UnknownPack { .. }),
        "{error}"
    );
    let message = error.to_string();
    assert!(
        message.contains("no-such-pack") && message.contains("pack-one"),
        "{message}"
    );
}

#[test]
fn a_missing_consumer_rule_path_is_named() {
    let temp = tempfile::tempdir().expect("temp dir");
    let missing = temp.path().join("absent.grit");

    let error = shell::read_rule_sources(std::slice::from_ref(&missing))
        .expect_err("a missing rule path is a refusal");

    assert!(
        matches!(error, GritlintError::RulePathMissing { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("absent.grit"), "{error}");
}

#[test]
fn a_pack_enabled_without_a_required_parameter_names_the_parameter() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = make_pack(
        temp.path(),
        "pack-one",
        &["condition"],
        &[("react.md", REACT_RULE)],
    );

    let error = error_of(decode::load_rules(
        &[pack_of(&dir)],
        &enabled("pack-one", &[]),
        Vec::new(),
        Path::new("gritlint.json"),
    ));

    assert!(
        matches!(error, GritlintError::MissingPackParam { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("condition"), "{error}");
}

#[test]
fn a_rule_calling_an_undefined_pattern_fails_to_compile_with_its_id() {
    let temp = tempfile::tempdir().expect("temp dir");
    let consumer = temp.path().join("broken.grit");
    write(
        &consumer,
        &format!("{FRONTMATTER}language json\nmissing_helper()\n"),
    );

    let error = error_of(decode::load_rules(
        &[],
        &BTreeMap::new(),
        sources(&[consumer]),
        Path::new("gritlint.json"),
    ));

    assert!(
        matches!(error, GritlintError::RuleCompile { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("local/broken"), "{error}");
}

#[test]
fn a_consumer_directory_does_not_load_rules_from_its_fixtures() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = temp.path().join("consumer");
    write(&dir.join("no-console.grit"), PLAIN_RULE);
    write(
        &dir.join("fixtures/no-console/bad/basic/hidden.grit"),
        PLAIN_RULE,
    );

    let rules = decode::load_rules(
        &[],
        &BTreeMap::new(),
        sources(std::slice::from_ref(&dir)),
        Path::new("gritlint.json"),
    )
    .expect("load the consumer rules");

    assert_eq!(ids(&rules), ["local/no-console"]);
}

#[test]
fn a_sibling_pattern_in_the_same_pack_is_callable() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = make_pack(
        temp.path(),
        "pack-one",
        &[],
        &[
            (
                "helper.grit",
                &format!(
                    "{FRONTMATTER}language json\npattern react_dependency() {{ `\"react\": $_` }}\nreact_dependency()\n"
                ),
            ),
            (
                "uses.grit",
                &format!("{FRONTMATTER}language json\nreact_dependency()\n"),
            ),
        ],
    );

    let rules = decode::load_rules(
        &[pack_of(&dir)],
        &enabled("pack-one", &[]),
        Vec::new(),
        Path::new("gritlint.json"),
    )
    .expect("a rule may call its sibling pattern");

    assert_eq!(ids(&rules), ["pack-one/helper", "pack-one/uses"]);
}

#[test]
fn a_rule_without_valid_frontmatter_refuses_naming_the_file_and_the_key() {
    let body = "language json\n`\"react\": $_`\n";
    let cases = [
        (
            "missing-reason",
            "---\nnextAction: Fix it.\n---\n",
            "reasonCode",
            FrontmatterProblem::Missing,
            "is missing",
        ),
        (
            "missing-action",
            "---\nreasonCode: some-reason\n---\n",
            "nextAction",
            FrontmatterProblem::Missing,
            "is missing",
        ),
        (
            "no-block",
            "",
            "reasonCode",
            FrontmatterProblem::Missing,
            "is missing",
        ),
        (
            "repeated",
            "---\nreasonCode: one\nreasonCode: two\nnextAction: Fix it.\n---\n",
            "reasonCode",
            FrontmatterProblem::Repeated,
            "is repeated",
        ),
        (
            "unknown",
            "---\nreasonCode: one\nnextAction: Fix it.\nlocation: `\"react\": $_`\n---\n",
            "location",
            FrontmatterProblem::Unknown,
            "is unknown; the keys are `reasonCode`, `nextAction`, `locate`",
        ),
        (
            "malformed",
            "---\nreasonCode: Bad_Code\nnextAction: Fix it.\n---\n",
            "reasonCode",
            FrontmatterProblem::NotKebabCase,
            "must be lowercase kebab-case, such as `missing-provenance`",
        ),
        (
            "empty",
            "---\nreasonCode:\nnextAction: Fix it.\n---\n",
            "reasonCode",
            FrontmatterProblem::Empty,
            "has no value",
        ),
    ];
    for (name, frontmatter, key, problem, text) in cases {
        let temp = tempfile::tempdir().expect("temp dir");
        let consumer = temp.path().join(format!("{name}.grit"));
        write(&consumer, &format!("{frontmatter}{body}"));

        let error = error_of(decode::load_rules(
            &[],
            &BTreeMap::new(),
            sources(&[consumer]),
            Path::new("gritlint.json"),
        ));

        assert!(
            matches!(
                &error,
                GritlintError::RuleFrontmatter { key: refused, problem: found, .. }
                    if refused == key && *found == problem
            ),
            "{name}: {error}"
        );
        let message = error.to_string();
        assert!(
            message.contains(&format!("{name}.grit")) && message.contains(key),
            "{name}: {message}"
        );
        assert!(message.contains(text), "{name}: {message}");
    }
}

#[test]
fn a_frontmatter_block_that_never_closes_refuses_naming_the_file() {
    let temp = tempfile::tempdir().expect("temp dir");
    let consumer = temp.path().join("unclosed.grit");
    write(
        &consumer,
        "---\nreasonCode: one\nnextAction: Fix it.\nlanguage json\n`\"react\": $_`\n",
    );

    let error = error_of(decode::load_rules(
        &[],
        &BTreeMap::new(),
        sources(std::slice::from_ref(&consumer)),
        Path::new("gritlint.json"),
    ));

    assert!(
        matches!(&error, GritlintError::RuleFrontmatterUnclosed { path } if *path == consumer),
        "{error}"
    );
}

#[test]
fn a_consumer_directory_with_two_rules_loads_both() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = temp.path().join("consumer");
    write(&dir.join("a.grit"), PLAIN_RULE);
    write(&dir.join("b.grit"), PLAIN_RULE);

    let rules = decode::load_rules(
        &[],
        &BTreeMap::new(),
        sources(std::slice::from_ref(&dir)),
        Path::new("gritlint.json"),
    )
    .expect("load the consumer rules");

    assert_eq!(ids(&rules), ["local/a", "local/b"]);
}

#[test]
fn a_pack_ignores_a_non_rule_file_in_its_rules_directory() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = make_pack(temp.path(), "pack-one", &[], &[("react.grit", PLAIN_RULE)]);
    write(&dir.join("rules/LICENSE"), "MIT\n");

    let pack = pack_of(&dir);

    assert_eq!(pack.sources().len(), 1, "{pack:?}");
    assert!(
        pack.sources()[0]
            .path()
            .as_path()
            .ends_with("rules/react.grit"),
        "{pack:?}"
    );
}

#[test]
fn a_heading_inside_an_example_block_stays_in_the_message() {
    let temp = tempfile::tempdir().expect("temp dir");
    let body = format!(
        "{FRONTMATTER}\n```\n# Example heading\n```\n\nProse that describes the rule.\n\n```grit\nlanguage json\n`\"console\": $_`\n```\n"
    );
    let dir = make_pack(
        temp.path(),
        "pack-one",
        &[],
        &[("heading.md", body.as_str())],
    );

    let rules = decode::load_rules(
        &[pack_of(&dir)],
        &enabled("pack-one", &[]),
        Vec::new(),
        Path::new("gritlint.json"),
    )
    .expect("load the pack");

    assert_eq!(
        rules.first().rule().message(),
        "```\n# Example heading\n```\n\nProse that describes the rule."
    );
}

#[test]
fn a_markdown_rule_with_two_grit_blocks_is_refused() {
    let temp = tempfile::tempdir().expect("temp dir");
    let body = format!(
        "{FRONTMATTER}```grit\nlanguage json\n`\"console\": $_`\n```\n\n```grit\nlanguage json\n`\"react\": $_`\n```\n"
    );
    let dir = make_pack(temp.path(), "pack-one", &[], &[("two.md", body.as_str())]);

    let error = error_of(decode::load_rules(
        &[pack_of(&dir)],
        &enabled("pack-one", &[]),
        Vec::new(),
        Path::new("gritlint.json"),
    ));

    assert!(
        matches!(error, GritlintError::RuleNoGritBlock { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("two.md"), "{error}");
}

#[test]
fn a_rule_whose_example_block_precedes_the_grit_block_keeps_its_body() {
    let temp = tempfile::tempdir().expect("temp dir");
    let body = format!(
        "{FRONTMATTER}```json\n{{\n  \"probe\": 1\n}}\n```\n\nExample prose.\n\n```grit\nlanguage json\n`\"console\": $_`\n```\nIt keeps this trailing prose.\n"
    );
    let dir = make_pack(
        temp.path(),
        "pack-one",
        &[],
        &[("example.md", body.as_str())],
    );

    let rules = decode::load_rules(
        &[pack_of(&dir)],
        &enabled("pack-one", &[]),
        Vec::new(),
        Path::new("gritlint.json"),
    )
    .expect("load the pack");

    assert_eq!(ids(&rules), ["pack-one/example"]);
    assert_eq!(
        rules.first().rule().body(),
        "language json\n`\"console\": $_`"
    );
}

#[test]
fn a_rule_cannot_call_a_pattern_declared_in_another_scope() {
    let temp = tempfile::tempdir().expect("temp dir");
    let shared = make_pack(
        temp.path(),
        "shared",
        &[],
        &[(
            "helper.grit",
            &format!(
                "{FRONTMATTER}language json\npattern shared_helper() {{ `\"react\": $_` }}\nshared_helper()\n"
            ),
        )],
    );
    let caller = temp.path().join("caller.grit");
    write(
        &caller,
        &format!("{FRONTMATTER}language json\nshared_helper()\n"),
    );

    let error = error_of(decode::load_rules(
        &[pack_of(&shared)],
        &enabled("shared", &[]),
        sources(std::slice::from_ref(&caller)),
        Path::new("gritlint.json"),
    ));

    assert!(
        matches!(error, GritlintError::RuleCompile { .. }),
        "{error}"
    );
    assert!(error.to_string().contains("local/caller"), "{error}");
}

#[test]
fn a_consumer_directory_loads_nested_rules_but_never_a_fixtures_subtree() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = temp.path().join("consumer");
    write(&dir.join("nested/deep.grit"), PLAIN_RULE);
    write(&dir.join("fixtures/hidden.grit"), PLAIN_RULE);

    let rules = decode::load_rules(
        &[],
        &BTreeMap::new(),
        sources(std::slice::from_ref(&dir)),
        Path::new("gritlint.json"),
    )
    .expect("load the consumer rules");

    assert_eq!(ids(&rules), ["local/deep"]);
}
