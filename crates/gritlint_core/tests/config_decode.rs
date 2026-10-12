#![allow(clippy::expect_used, clippy::disallowed_methods)]

mod common;

use gritlint_core::decode::{self, GritlintConfig};
use gritlint_core::shell;
use std::fs;
use std::path::Path;

#[test]
fn the_configuration_schema_is_the_committed_schema() {
    let packs = common::packs_in(&common::bundled_packs_dir());
    let schema = GritlintConfig::schema_for_packs(&packs);
    let rendered = format!(
        "{}\n",
        serde_json::to_string_pretty(&schema).expect("the schema serializes")
    );

    assert_eq!(
        rendered,
        include_str!("../../../npm/gritlint/configuration_schema.json")
    );
}

#[test]
fn a_config_pack_entry_becomes_a_typed_pack_name_and_parameters() {
    let config = GritlintConfig::parse(
        r#"{"packs": {"npm-provenance": {"repositoryUrl": "https://example.com/repo"}}}"#,
    )
    .expect("parse the config");

    let enabled =
        decode::enabled_packs(&config, Path::new("gritlint.json")).expect("enable the pack");

    assert_eq!(enabled.len(), 1);
    let (pack, params) = enabled.iter().next().expect("the enabled pack");
    assert_eq!(pack.as_str(), "npm-provenance");
    assert_eq!(params.len(), 1);
    let (param, value) = params.iter().next().expect("the pack's one parameter");
    assert_eq!(param.as_str(), "repositoryUrl");
    assert_eq!(value, "https://example.com/repo");
}

#[test]
fn a_pack_parameter_that_is_not_an_identifier_refuses_naming_the_key_the_config_and_the_reason() {
    let config = GritlintConfig::parse(r#"{"packs": {"npm-provenance": {"not-a-name": "x"}}}"#)
        .expect("parse the config");

    let error = decode::enabled_packs(&config, Path::new("gritlint.json"))
        .expect_err("a parameter that is not an identifier is refused");

    let message = error.to_string();
    assert!(message.contains("not-a-name"), "{message}");
    assert!(message.contains("gritlint.json"), "{message}");
    assert!(
        message.contains("the name is not a GritQL identifier"),
        "{message}"
    );
}

#[test]
fn an_empty_pack_parameter_name_refuses_saying_the_name_is_empty() {
    let config = GritlintConfig::parse(r#"{"packs": {"npm-provenance": {"": "x"}}}"#)
        .expect("parse the config");

    let error = decode::enabled_packs(&config, Path::new("gritlint.json"))
        .expect_err("an empty parameter name is refused");

    let message = error.to_string();
    assert!(message.contains("gritlint.json"), "{message}");
    assert!(message.contains("the name is empty"), "{message}");
}

#[test]
fn is_file_is_true_only_for_an_existing_regular_file() {
    let temp = tempfile::tempdir().expect("temp dir");
    let config = temp.path().join("gritlint.json");
    fs::write(&config, "{}\n").expect("write the config");

    assert!(shell::is_file(&config), "a written file is a file");
    assert!(!shell::is_file(temp.path()), "a directory is not a file");
    assert!(
        !shell::is_file(&temp.path().join("absent.json")),
        "an absent path is not a file"
    );

    fs::remove_file(&config).expect("remove the config");
    assert!(!shell::is_file(&config), "a removed file is not a file");
}
