#![allow(clippy::expect_used, clippy::disallowed_methods)]

use gritlint_core::decide;
use gritlint_core::decode;
use gritlint_core::domain::{EmbeddedFile, RelPath};
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

#[derive(serde::Deserialize)]
struct Manifest {
    #[serde(rename = "requiredParams")]
    required_params: Vec<String>,
}

fn embedded_files(packs: &Path) -> Vec<EmbeddedFile> {
    let tree = shell::read_tree(packs).expect("read the bundled packs");
    tree.files()
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
        .collect()
}

fn entry_names(dir: &Path) -> Vec<String> {
    fs::read_dir(dir)
        .expect("list the directory")
        .map(|entry| {
            entry
                .expect("a readable directory entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect()
}

#[test]
fn every_bundled_pack_decodes_into_its_committed_rule_files() {
    let packs = packs_dir();
    let decoded =
        decode::embedded_packs(&embedded_files(&packs)).expect("decode the bundled packs");

    let mut expected_names: Vec<String> = subdirectories(&packs)
        .iter()
        .map(|pack| {
            pack.file_name()
                .expect("a pack directory has a name")
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    expected_names.sort();
    let names: Vec<String> = decoded
        .iter()
        .map(|pack| pack.name().as_str().to_owned())
        .collect();
    assert_eq!(
        names, expected_names,
        "the decoded packs are the packs/ directories"
    );

    for pack in &decoded {
        let name = pack.name().as_str();

        let mut expected_sources: Vec<String> = entry_names(&packs.join(name).join("rules"))
            .into_iter()
            .map(|entry| format!("{name}/rules/{entry}"))
            .collect();
        expected_sources.sort();
        let sources: Vec<String> = pack
            .sources()
            .iter()
            .map(|source| source.path().as_path().to_string_lossy().into_owned())
            .collect();
        assert_eq!(sources, expected_sources, "pack `{name}` rules");

        for source in pack.sources() {
            let path = source.path().as_path();
            let committed =
                fs::read_to_string(packs.join(path)).expect("read the committed rule file");
            assert_eq!(source.text(), committed, "{}", path.display());
        }

        let manifest_text =
            fs::read_to_string(packs.join(name).join("pack.json")).expect("read the manifest");
        let manifest: Manifest =
            serde_json::from_str(&manifest_text).expect("parse the committed manifest");
        let required: Vec<String> = pack
            .required_params()
            .iter()
            .map(|param| param.as_str().to_owned())
            .collect();
        assert_eq!(
            required, manifest.required_params,
            "pack `{name}` required params"
        );
    }
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
