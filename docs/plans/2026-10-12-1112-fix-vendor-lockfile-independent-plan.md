---
title: Vendor the cargo dependencies without a hash that covers workspace versions - Plan
type: fix
date: 2026-10-12
topic: vendor-lockfile-independent
artifact_contract: ce-unified-plan/v1
product_contract_source: root-ruling
execution: code
source_commit: systemfsoftware/gritlint@a84c2cc03
---

# Vendor the cargo dependencies without a hash that covers workspace versions - Plan

## Goal Capsule

- **Objective:** main builds again, and the next version PR cannot break the vendor. Release run 38267387233 and Mutants run 38267386759 failed with `hash mismatch in fixed-output derivation '/nix/store/ki2painxnylmrf5837zzw3c8wg7v4qmw-cargo-deps-vendor-staging.drv'` after #18 bumped the workspace to 0.2.0.
- **Means:** replace `rustPlatform.fetchCargoVendor` in `nix/gritlint.nix` with crane's `vendorCargoDeps`, which carries no fixed-output hash. Every flake output keeps its name and contents.
- **Product authority:** the root's P0 ruling of 2026-10-12: fix forward from main, with a hashless vendor that reads only third-party crates. A hand-edited hash is rejected because the next version PR breaks it again.
- **Proof:** a scratch commit made by the real bot command (`version-management bump` over a patch changeset) fails on main's vendor and builds the binaries and tarball on this fix. The scratch commit is never pushed.

## Product Contract

### Problem Frame

The pinned nixpkgs `fetch-cargo-vendor-util.py` copies the whole `Cargo.lock` into the fixed-output staging derivation (`create_vendor_staging`, line 156: `shutil.copy(lockfile_path, out_dir / "Cargo.lock")`). The lock holds the workspace members' versions, so a version bump changes the staging output and fails its declared `hash`. Feeding the vendor a lock-only source (the 2026-10-10 lesson) does not help, because the members are in the lock. A developer store that already holds the old output never rebuilds it, so the bump built green locally and failed in CI.

### Requirements

- R1. The vendor's derivation does not depend on any workspace member's version: two trees that differ only by a `version-management bump` evaluate `gritlint-unwrapped.vendor` to the same drvPath.
- R2. No fixed-output hash covers the vendor. Registry crates are fetched by their `Cargo.lock` checksums; the GritQL engine is fetched at its locked rev.
- R3. The vendored engine crates are the copies the engine's own manifests build (`resources/language-metavariables`), whatever the filesystem's directory order.
- R4. Every existing output and check builds: the static binaries, the tarball, the sandboxed CLI and every `checks.<system>` entry. `vendor/config.toml` still serves the `cargo deny` and mutants jobs, which copy it into `CARGO_HOME`.
- R5. No workflow, action or timeout changes.

### Assumptions This Plan Breaks

- A lock-only vendor source makes the vendor depend only on third-party crates. It does not: the members' versions are in the lock, and the staging output copies the lock.
- crane's git vendoring cannot serve this engine (KTD10 of the standalone plan). It can, once the checkout is prepared through `overrideVendorGitCheckout`.
- A green local build proves a vendor hash. A store that holds the old fixed output never refetches it; only a fresh store or `--rebuild` does.

### Tests

No permanent test is admitted. The version-churn proof is a scratch commit that is never pushed; every existing flake check runs unchanged.

### Scope Boundaries

- The engine pin, `Cargo.lock` and the grammar patch stay as they are.
- `docs/plans/2026-10-11-1301-feat-gritlint-standalone-plan.md` KTD10 records the fetchCargoVendor decision as it was made; it is not rewritten.

## Key Technical Decisions

- KTD1. **crane `vendorCargoDeps`, not `importCargoLock`.** Both are hashless. Pinned nixpkgs `import-cargo-lock.nix` finds a git crate by `cargo metadata` on the repository root and otherwise takes the first `find` hit by name (lines 235-248). The engine's grammar crates are not workspace members of the gritql root, so it would take the first of two copies, the bug `gritlint.nix` already records. It also links the whole lock into its vendor directory (line 318). crane (v0.24.0) reads only `name`, `version`, `source` and `checksum` from the lock (`vendorMultipleCargoDeps.nix`), skips entries with no `source`, and offers `overrideVendorGitCheckout` to prepare the checkout.
- KTD2. **Prepare the checkout before crane scans it.** crane keeps the first `find` hit per name and version (`downloadCargoPackageFromGit.nix`). Sixteen grammar crates appear in both `resources/language-metavariables` and `resources/language-submodules`; the engine's manifests point at the first. `preInstall` deletes `resources/language-submodules`.
- KTD3. **Delete the nested grammar manifests.** crane copies the files `cargo package --list` names, and cargo leaves out any directory holding its own `Cargo.toml`. `tree-sitter-php` (`php/`, `php_only/`) and `tree-sitter-md` (`tree-sitter-markdown/`, `tree-sitter-markdown-inline/`) keep their sources there, so the dependency build failed on `./php/src/parser.c`. `preInstall` deletes every `Cargo.toml` at depth 3 or more under `resources/language-metavariables`; no lock entry names one.
- KTD4. **Copy only the engine's source directory.** crane links each crate from its own store path, so `../../../resources` from marzano-language's `src` would leave the vendor. `vendor` dereferences the source directory holding `marzano-language-*`, links the registry, adds `resources/node-types` and rewrites crane's `config.toml` to its own path.

## Implementation Units

### U1. crane vendor

- **Requirements:** R1, R2, R3, R4.
- **Files:** `nix/gritlint.nix`.
- **Verification:** `nix build` of the vendor, both dependency derivations, every `checks.x86_64-linux` entry, `gritlint-binaries` and `gritlint-tarball`. The vendored engine crates match fetchCargoVendor's byte for byte on every file both trees hold; crane's tree drops only files outside each crate's `include`.

### U2. Living docs

- **Files:** `docs/solutions/tooling-decisions/gritlint-conventions-platform.md`, `docs/solutions/performance-issues/gritql-target-language-debug-key-embeds-node-table.md`, `vendor/tree-sitter-typescript/README.md`, and the 2026-10-10 vendor lesson.
- **Change:** remove the `cargoDeps.hash` re-pin steps and the `source-git-0` path.

### U3. Version-churn proof

- **Verification:** in a scratch worktree, a patch changeset plus `nix develop --command version-management bump` (release-tools at `b9b4a1d`). Main's vendor fails with a hash mismatch in a fresh store; on this fix the same bump builds `gritlint-binaries` and `gritlint-tarball`, and the vendor drvPath equals the unbumped one.
