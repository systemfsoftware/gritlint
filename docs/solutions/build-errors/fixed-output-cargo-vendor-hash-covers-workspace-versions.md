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

Vendor with crane's `vendorCargoDeps`, which has no fixed-output hash. It keeps only `name`, `version`, `source` and `checksum` of each lock entry and drops entries with no `source`, which are the workspace members. Registry crates are fetched by their lock checksums, and git crates by `builtins.fetchGit` at the locked rev:

```nix
cargoDeps = craneLib.vendorCargoDeps {
  cargoLock = ../Cargo.lock;
  overrideVendorGitCheckout = packages: checkout:
    if lib.any (p: lib.hasPrefix "git+${engineRepo}?" p.source) packages then
      checkout.overrideAttrs (old: {
        preInstall = (old.preInstall or "") + ''
          rm -r resources/language-submodules
          find resources/language-metavariables -mindepth 3 -name Cargo.toml -delete
        '';
      })
    else checkout;
};
```

The GritQL checkout needed two changes before crane could vendor it:

- crane keeps the first `find` hit for each name and version, and `find` order depends on the filesystem. The engine has 16 grammar crates twice: the metavariable copies its manifests build, and the upstream copies in its git submodules. Deleting the submodule copies leaves one candidate for each.
- crane copies the files `cargo package --list` names. Cargo leaves out every directory that has its own `Cargo.toml`. `tree-sitter-php` and `tree-sitter-md` keep their grammar sources in subdirectories with their own manifests, so the dependency build failed with `./php/src/parser.c: No such file or directory`. Deleting the nested manifests fixes this, and no lock entry names any of them.

`nixpkgs`' `importCargoLock` (with `allowBuiltinFetchGit`) is hashless as well, but it doesn't work for this engine. It looks a git crate up with `cargo metadata` on the repository root, and the grammar crates are not workspace members there. It then takes the first `find` hit by name, and it links the whole lock into its vendor directory.

## Why This Works

A fixed-output hash pins the bytes of its output, so it holds only while every input that reaches the output is one the hash is meant to pin. The bot rewrites the workspace's own versions on every release. Any vendor whose output contains those versions, such as a copied lock, will break on the next release.

- wrong: a fixed-output vendor whose output holds a copy of `Cargo.lock`, so its hash depends on the workspace members.
- right: a vendor built only from third-party entries (name, version, source, checksum), each fetched by its own content address: the lock checksum for a registry crate, the locked rev for a git crate.

## Prevention

- When a vendor is a fixed-output derivation, check that its output depends only on third-party lock entries. Gate: make a scratch commit with the real bot command (`nix develop --command <release-tools>/bin/version-management bump` over a patch changeset), then check that `gritlint-unwrapped.vendor.drvPath` evaluates to the same path before and after it. Use a fresh store (`nix build --store /tmp/<dir> .#packages.x86_64-linux.gritlint-binaries`) so that an old output already in the store can't mask a failure.
- When the engine is re-pinned, read the cargo-git derivation's log (`nix log`, on the `cargo-git-…gritql…` derivation among the vendor's requisites). A `skipping duplicate package` line for a locked crate means a new duplicate has appeared. The engine's two `xtask` crates are the expected exception: no lock entry names them.

## Related Issues

- [fetch-cargo-vendor-given-whole-source-rebuilds-crane-deps.md](../performance-issues/fetch-cargo-vendor-given-whole-source-rebuilds-crane-deps.md): the lock-only vendor source that this replaces
- `docs/plans/2026-10-12-1112-fix-vendor-lockfile-independent-plan.md`
