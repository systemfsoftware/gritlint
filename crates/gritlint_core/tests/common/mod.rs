#![allow(dead_code, clippy::expect_used)]

use gritlint_core::decode;
use gritlint_core::domain::{EmbeddedFile, Pack, RelPath};
use gritlint_core::shell;
use std::path::{Path, PathBuf};

pub fn bundled_packs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packs")
}

pub fn embedded_files(dir: &Path) -> Vec<EmbeddedFile> {
    let tree = shell::read_tree(dir).expect("read the packs");
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

pub fn packs_in(dir: &Path) -> Vec<Pack> {
    decode::embedded_packs(&embedded_files(dir)).expect("decode the packs")
}
