---
title: Vendor the cargo dependencies so workspace version bumps cannot move the vendor - Plan
type: fix
date: 2026-10-12
topic: vendor-workspace-version-independent
artifact_contract: ce-unified-plan/v1
product_contract_source: root-ruling
execution: code
source_commit: systemfsoftware/gritlint@a84c2cc03
supersedes: docs/plans/2026-10-12-1112-fix-vendor-lockfile-independent-plan.md
---

# Vendor the cargo dependencies so workspace version bumps cannot move the vendor - Plan

## Goal Capsule

- **Objective:** get main building again after the version bump, and make sure no future workspace version change can trigger the cargo vendor failure again. Release run 38267387233 and Mutants run 38267386759 failed with `hash mismatch in fixed-output derivation '/nix/store/ki2painxnylmrf5837zzw3c8wg7v4qmw-cargo-deps-vendor-staging.drv'` after #18 bumped the workspace to 0.2.0.
- **Means:** `nix/gritlint.nix` vendors with crane's `vendorCargoDeps`. Registry crates come by their `Cargo.lock` checksums. The GritQL engine comes from one fixed-output fetch keyed to its locked rev. Two flake checks pin the vendor's laws.
- **Product authority:** the root's P0 ruling of 2026-10-12 says to fix forward and not to hand-update the version-coupled hash. The root later ruled that the engine checkout carries its own rev-keyed hash (the "Engine-only outputHashes" answer) and that both law checks land in this PR (the "Add both checks in #19" answer).
- **Proof:** a scratch commit made with the bot's real command (`version-management bump` over a patch changeset) fails on main's vendor in a fresh store. On this fix, the same commit builds the binaries and the tarball. Neither scratch commit is pushed.

## Product Contract

### Problem Frame

The pinned nixpkgs `fetch-cargo-vendor-util.py` copies the whole `Cargo.lock` into the fixed-output staging derivation. In `create_vendor_staging` that is `shutil.copy(lockfile_path, out_dir / "Cargo.lock")`. The lock holds the workspace members' versions, so a version bump changes the staging output and fails its declared `hash`. A developer store that already holds the old output never rebuilds it.

### Requirements

- R1. The vendor's derivation does not depend on any workspace member's version. Given the lock with every workspace member's version moved, the vendor function returns the same drvPath. Pinned by `gritlint-vendor-ignores-workspace-versions`.
- R2. No hash covers a workspace member. Registry crates are fetched by their lock checksums. The engine is fetched once, as a fixed-output fetch whose hash (`engineHash`) changes only when the engine is re-pinned.
- R3. Every vendored engine crate is the copy that the engine's own marzano-language manifest names by path, whatever the filesystem's directory order. Pinned by `gritlint-vendor-engine-copies`.
- R4. Every existing output and check still builds. `vendor/config.toml` still serves the `cargo deny` and mutants jobs.
- R5. No workflow, action or timeout changes.
- R6. The engine fetch does not run at evaluation time; CI evaluates before it restores the store cache. Pinned by an evaluation assertion in `gritlint.nix`: the engine is the lock's only git source and is not percent-encoded, so crane always finds its `outputHashes` entry.

### Assumptions This Plan Breaks

- That a lock-only vendor source makes the vendor depend only on third-party crates. It does not: the members' versions are in the lock, and the staging output copies the lock.
- That a hashless vendor is free. Crane's hashless git path is an eval-time `builtins.fetchGit` with submodules. Measured on run 38273376142, it added about 570 s to each Nix job.
- That a green local build proves a vendor hash. A store that already holds the old fixed output never refetches it.

### Pack and Constitution Compliance

All five declared packs at `02bf9653` were loaded, along with the constitution at the same ref. Only one pack rule matches this change, `boundary-testing/pin-dependency-semantics`, which applies when code relies on subtle vendor behaviors. The change relies on three crane behaviors. Crane drops sourceless lock entries, it keeps the first `find` hit per name and version, and `cargo package --list` skips nested manifests. That rule says such assumptions must be executable pins that fire again when the lockfile changes, so U4 adds them. The other pack rules apply only to TypeScript/Effect cells, schemas, mutation targets or package exports, and none of those is touched here. The constitution laws that apply are CONST-S1 (fix the root cause: the version-coupled hash is removed, not re-pinned), CONST-T3 and CONST-T10 (each new check fails on a fixture and takes its oracle from outside the code under test), CONST-T12 (the engine check selects crates by the engine manifest's path dependencies, not by directory) and CONST-E7 (no existing check is weakened).

### Scope Boundaries

- The engine pin, `Cargo.lock` and the grammar patch stay as they are.
- `docs/plans/2026-10-11-1301-feat-gritlint-standalone-plan.md` KTD10 stays as the decision record it was.

### Tests

The test-layer gate admits two tests. R1 is a deterministic regression pin for the #18 failure. R3 is a differential check of the vendored crates against an independent reference, the engine's manifest-named copies. Both live as flake checks next to `gritlint-vendor-sync`. No other permanent test is admitted.

## Key Technical Decisions

- KTD1. **crane `vendorCargoDeps`, not `importCargoLock`.** The pinned nixpkgs `import-cargo-lock.nix` falls back to the first `find` hit by name when `cargo metadata` on the gritql root does not list a crate. The grammar crates are not workspace members there. It also links the whole lock into its vendor. Crane (v0.24.0) reads only `name`, `version`, `source` and `checksum`, skips entries with no `source`, and offers `overrideVendorGitCheckout`.
- KTD2. **Delete the upstream grammar copies before crane scans the checkout.** Sixteen grammar crates appear in both `resources/language-metavariables` and `resources/language-submodules`, and crane keeps the first `find` hit.
- KTD3. **Delete the nested grammar manifests.** Cargo leaves out any directory that holds its own `Cargo.toml`, which dropped `tree-sitter-php`'s `php/src`.
- KTD4. **Copy only the engine's source directory into the vendor.** marzano-language's `include_str!("../../../resources/…")` must resolve inside the vendor.
- KTD5. **The engine gets a rev-keyed fixed-output fetch (`outputHashes`), shared with `engineSource`.** Without it, crane's `builtins.fetchGit` cloned every engine submodule at evaluation time on every CI run (eval step 50 s → 618/625 s). A `fetchgit` with `fetchSubmodules` runs at build time (about 60 s cold) and is served from the store cache. Its hash is keyed by the lock's engine source string, so version PRs never move it. `engineSource` uses the identical `fetchgit` call, so there is a single derivation.
- KTD6. **The vendor is a function of the parsed lock (`vendorFor`).** That lets a check vendor a lock whose workspace members moved.

## Implementation Units

### U1. crane vendor

- **Requirements:** R1–R4, R6. **Files:** `nix/gritlint.nix`.

### U2. Living docs

- **Files:** `docs/solutions/tooling-decisions/gritlint-conventions-platform.md`, `docs/solutions/performance-issues/gritql-target-language-debug-key-embeds-node-table.md`, `vendor/tree-sitter-typescript/README.md`, the 2026-10-10 vendor lesson, and the new vendor lesson.

### U3. Version-churn proof

- **Verification:** in a scratch worktree, a patch changeset followed by `nix develop --command version-management bump` (release-tools at `b9b4a1d`). Main's vendor fails with a hash mismatch in a fresh store. On this fix the same bump builds `gritlint-binaries` and `gritlint-tarball`.

### U4. Vendor law checks

- **Requirements:** R1, R3, R6. **Files:** `nix/gritlint-vendor-checks.nix`, `nix/gritlint-vendor-engine-copies.py`, `nix/gritlint-checks.nix`, `nix/gritlint.nix`, `flake.nix`.
- **Verification:** both checks pass on this branch. Scratch fixtures that leave the source alone make each one fail. R1 fails on main's `fetchCargoVendor` vendor. R3 fails on a vendor whose `tree-sitter-css` is the upstream submodule copy, on one without `tree-sitter-php`'s `php/src`, on one without `marzano-core`, on a lock that pins `tree-sitter-json` at a version its manifest does not declare, and on a lock without `tree-sitter-css`. A scratch copy of the tree whose lock holds a second engine source fails evaluation.
