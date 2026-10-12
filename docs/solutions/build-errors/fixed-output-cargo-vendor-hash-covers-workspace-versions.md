---
title: A fixed-output cargo vendor hash covers the workspace's own versions, so every version PR breaks it
date: 2026-10-12
category: build-errors
module: gritlint
problem_type: build_error
component: tooling
symptoms:
  - "error: hash mismatch in fixed-output derivation '/nix/store/ki2painxnylmrf5837zzw3c8wg7v4qmw-cargo-deps-vendor-staging.drv'"
  - "Release and Mutants failed on main right after the version PR (#18) merged; every flake output depends on the vendor"
  - "The same commit built green in a developer store that already held the old fixed output"
root_cause: config_error
resolution_type: config_change
severity: critical
related_components:
  - nix/gritlint.nix
  - Cargo.lock
  - release.jsonc
tags: [nix, crane, fetchcargovendor, vendorcargodeps, cargo-lock, fixed-output-derivation, version-bump, release]
---

# A fixed-output cargo vendor hash covers the workspace's own versions, so every version PR breaks it

## Problem

The release bot's version PR (#18) moved the workspace from 0.1.0 to 0.2.0. Its `cargo` surface (`release.jsonc`) rewrites `[workspace.package]` in `Cargo.toml` and the four workspace members' entries in `Cargo.lock`. gritlint's Nix package vendored with `rustPlatform.fetchCargoVendor` and a pinned `hash`, so after the merge every output on main failed on `cargo-deps-vendor-staging`.

## Root cause

The pinned nixpkgs' `fetch-cargo-vendor-util create-vendor-staging` copies the whole lock into the fixed-output staging derivation (`create_vendor_staging`: `shutil.copy(lockfile_path, out_dir / "Cargo.lock")`). The workspace members' versions are part of that output, so the hash changes with every version bump, even though no third-party crate moved. Giving the fetcher a source with only `Cargo.lock` (the 2026-10-10 lesson) does not help, because the members are in the lock.

A local build hides this. A fixed-output path is named by its declared hash, so a store that already holds the old output never runs the new staging derivation.

## Solution

Vendor with crane's `vendorCargoDeps`. It keeps only `name`, `version`, `source` and `checksum` of each lock entry and drops entries with no `source`, which are the workspace members. Registry crates are fetched by their lock checksums. The engine is one fixed-output fetch keyed by its lock source string, so its hash moves only when the engine is re-pinned:

```nix
vendorFor = lock: craneLib.vendorCargoDeps {
  cargoLockParsed = lock;
  outputHashes.${engineSourceId} = engineHash;   # "git+<repo>?rev=R#R"
  overrideVendorGitCheckout = packages: checkout:
    if lib.any (p: p.source == engineSourceId) packages then
      checkout.overrideAttrs (old: {
        preInstall = (old.preInstall or "") + ''
          rm -r resources/language-submodules
          find resources/language-metavariables -mindepth 3 -name Cargo.toml -delete
        '';
      })
    else checkout;
};
```

A fully hashless vendor was tried first and dropped. Without `outputHashes`, crane fetches the engine with `builtins.fetchGit` (submodules included, all refs) at evaluation time. In CI, evaluation runs before the store cache is restored, so the step that evaluates every output went from about 50 s to 618 s (x86_64) and 625 s (aarch64), almost all of it spent cloning submodules. With the hash, crane uses `fetchgit`, which runs at build time, takes about 60 s on a cold store, and is served from the store cache after that.

The GritQL checkout needed two changes before crane could vendor it:

- crane keeps the first `find` hit for each name and version, and `find` order depends on the filesystem. The engine has 16 grammar crates twice: the metavariable copies its manifests build, and the upstream copies in its git submodules. Deleting the submodule copies leaves one candidate for each.
- crane copies the files `cargo package --list` names. Cargo leaves out every directory that has its own `Cargo.toml`. `tree-sitter-php` and `tree-sitter-md` keep their grammar sources in subdirectories with their own manifests, so the dependency build failed with `./php/src/parser.c: No such file or directory`. Deleting the nested manifests fixes this, and no lock entry names any of them.

`nixpkgs`' `importCargoLock` (with `allowBuiltinFetchGit`) is hashless as well, but it doesn't work for this engine. It looks a git crate up with `cargo metadata` on the repository root, and the grammar crates are not workspace members there. It then takes the first `find` hit by name, and it links the whole lock into its vendor directory.

## Why This Works

A fixed-output hash pins the bytes of its output, so it holds only while every input that reaches the output is one the hash is meant to pin. The bot rewrites the workspace's own versions on every release. Any vendor whose output contains those versions, such as a copied lock, will break on the next release.

- wrong: a fixed-output vendor whose output holds a copy of `Cargo.lock`, so its hash depends on the workspace members.
- right: a vendor built only from third-party entries (name, version, source, checksum), each fetched by its own content address: the lock checksum for a registry crate, and for the git engine a hash keyed by its locked rev.

## Prevention

- Pin the law, not the instance. The flake check `gritlint-vendor-ignores-workspace-versions` gives `vendorFor` the lock with every workspace member's version moved and fails evaluation if the vendor's drvPath changes. Run against main's `fetchCargoVendor` vendor, the same check fails.
- Pin the grammar choice. `gritlint-vendor-engine-copies` checks that each locked crate marzano-language's manifest takes by path is vendored once, with every file byte-identical to that copy at the pinned rev. A vendor that takes `tree-sitter-css` from the upstream submodule copy fails it on the grammar's generated parser and its `grammar.js`.
- A fixed-output path is named by its declared hash, so a store that already holds the old output hides a stale hash. Prove a vendor change in a fresh store (`nix build --store /tmp/<dir> …`), not in a developer store.

## Related Issues

- [fetch-cargo-vendor-given-whole-source-rebuilds-crane-deps.md](../performance-issues/fetch-cargo-vendor-given-whole-source-rebuilds-crane-deps.md): the lock-only vendor source that this replaces
- `docs/plans/2026-10-12-1740-fix-vendor-workspace-version-independent-plan.md`
