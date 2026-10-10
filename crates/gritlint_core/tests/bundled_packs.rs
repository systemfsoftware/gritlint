#![allow(clippy::expect_used, clippy::disallowed_methods)]

use gritlint_core::decide;
use gritlint_core::decode;
use gritlint_core::shell;
use std::fs;
use std::path::{Path, PathBuf};

fn packs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs")
}

fn subdirectories(dir: &Path) -> Vec<PathBuf> {
    fs::read_dir(dir)
        .map(|entries| {
            entries
                .map(|entry| entry.expect("a readable directory entry").path())
                .filter(|path| path.is_dir())
                .collect()
        })
        .unwrap_or_default()
}

fn committed_case_count(packs: &Path) -> usize {
    subdirectories(packs)
        .iter()
        .flat_map(|pack| subdirectories(&pack.join("fixtures")))
        .flat_map(|rule| [rule.join("good"), rule.join("bad")])
        .map(|kind| subdirectories(&kind).len())
        .sum()
}

#[test]
fn every_bundled_case_passes() {
    let packs = packs_dir();
    let tree = shell::read_tree(&packs).expect("read the bundled packs");
    let layout = decode::fixture_layout(&tree).expect("lay out the bundled fixtures");
    let outcome = decide::run_fixtures(&layout).expect("run the bundled fixtures");

    let failures: Vec<String> = outcome
        .failures()
        .iter()
        .map(|failure| {
            format!(
                "{} {}: {}",
                failure.rule_id(),
                failure.case(),
                failure.detail()
            )
        })
        .collect();
    assert!(
        failures.is_empty(),
        "{} bundled case(s) failed:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(
        outcome.results().len(),
        committed_case_count(&packs),
        "every committed case directory must produce exactly one result"
    );
}
