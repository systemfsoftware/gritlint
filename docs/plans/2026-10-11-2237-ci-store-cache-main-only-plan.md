---
title: Save the Nix store cache on main only - Plan
type: ci
date: 2026-10-11
topic: store-cache-main-only
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# Save the Nix store cache on main only - Plan

## Goal Capsule

- **Objective:** every pull request run and every push to `main` starts from `main`'s newest Nix store bundle, so a tests-only pull request finishes well inside the 10-minute PR budget instead of rebuilding the GritQL dependency tree cold.
- **Means:** only runs on `refs/heads/main` export and save the bundle. Every other run restores and never saves (KTD1).
- **Authority:** this plan, then the sub-conductor rulings of 2026-10-11. KTD1 replaces the saving sentence of KTD13 in `docs/plans/2026-10-10-1309-feat-gritlint-standalone-plan.md` (R7). That plan stays frozen.
- **Stop conditions:** stop and ask before adding any tool or action, changing an action pin, adding a credential or an external cache, or changing `timeout-minutes`.
- **Execution profile:** one unit, one commit, on `ci/store-cache-main-only` cut from `origin/main` (`d96dc51c1`). The PR into `main` carries the Verification Contract numbers available before merge, and a post-merge update carries the rest. The operator merges.

---

## Product Contract

### Summary

The two steps that export and save the Nix store bundle in `.github/workflows/ci.yml` gain a `main`-only condition. Pull request runs keep the restore step, which already falls back to `main` through the `nix-store-<system>-` prefix. They skip the export and the save.

### Problem Frame

On 2026-10-11 the repository's Actions cache held 10,545,221,338 bytes in 6 entries, over GitHub's 10 GB limit. All 6 belonged to `refs/pull/15/merge` and `refs/pull/16/merge`. `main` had none, because they had been evicted. `ci.yml` saves a full store export from every run whose key misses. The key hashes every output path, so nearly every push misses. GitHub scopes a pull request's cache to its merge ref: only re-runs of that same pull request can restore it. Every pull request therefore pays up to 4 GB of cache space for an entry no other run can use, and it evicts the one bundle every run could use.

Result: #16, a tests-only pull request, restored nothing. Its x86_64 job took 592 s (run 38149555075: `Nix store cache` 0 s, build 486 s, export 35 s, save 9 s), against the 10-minute PR budget. A warm `main` push of a tests-only change took 237 s on x86_64 (run 38130956734).

### Requirements

**Restore**

- R1. A run whose own ref holds no store bundle restores `main`'s newest bundle for its system, whenever `main` has one.
- R2. A push to `main` restores the previous `main` bundle for its system, whenever one exists.

**Save**

- R3. Only runs on `refs/heads/main` export and save a store bundle. A pull request run, or a run on any other branch, neither exports nor saves.
- R4. After the change merges, every new cache entry has ref `refs/heads/main`. Pull request runs add nothing to the cache footprint.

**Measurement**

- R5. CI cost is reported from the jobs API as per-job wall time on x86_64 and aarch64, for 4 runs: a cold and a warm pull request run, where the warm run restores `main`'s bundle, and a cold and a warm push to `main`. The budgets are measured, not enforced. `timeout-minutes` stays at 60, and a job over 10 minutes is listed with its time, not cut.

**Constraints**

- R6. No new tool or action. Every action keeps its full-SHA pin. No credentials and no external cache.
- R7. KTD1 replaces the sentence of the standalone plan's KTD13 that begins "Saving happens on every ref". The standalone plan is not edited, because it is the contract its executed units were verified against.

### Acceptance Examples

- AE1. **Covers R1, R3.** Given `main` holds bundle `nix-store-x86_64-linux-<h1>` and a pull request ref holds none, when the pull request's x86_64 job runs with key `<h2>`, then `Nix store cache` restores `<h1>` by prefix, the build runs, and `Export the unsigned store paths` and `Save the Nix store cache` are skipped.
- AE2. **Covers R2, R3.** Given `main` holds `<h1>`, when a push to `main` produces key `<h2>`, then the job restores `<h1>`, builds, and saves `<h2>` with ref `refs/heads/main`.
- AE3. **Covers R3.** Given a push to `main` whose key exactly matches an existing `main` entry, export and save are skipped, as they are today.
- AE4. **Covers R1.** Given a pull request ref that still holds an entry saved before this change (#15, #16), its run restores its own entry first, because GitHub searches the current ref before the default branch. That entry is the closest store to the pull request's own code, so this is accepted rather than worked around.

### Key Decisions

- **Save only on `main`.** The alternatives under Scope Boundaries each leave pull request entries that no other run can restore. Governs R3, R4.

### Scope Boundaries

- Considered and not built:
  - **Delete a pull request's superseded entry on each push.** One entry pair stays per open pull request: (open PRs + 1) x 4.0 GB, which is 12.0 GB with today's 2 open PRs, so `main` is evicted again. It needs `actions: write` on pull request runs, and fork pull requests never get that. Evidence that would change the call: none, because the entries stay unrestorable by any other ref.
  - **Narrow the export to the paths a rebuild reads.** Measured against `0dfe7947` on x86_64: 271 exported paths, 5.50 GB of NAR, of which 1.98 GB (36%) a tests-only rebuild never reads (both vendor staging trees at 0.5 GB each, toolchain tarballs, sources). That would make about 2.55 GB per generation. Kept per ref, the bundles still can't be restored by another ref, and main plus 3 open PRs is 10.2 GB, still over the limit. Added to KTD1, it buys a third `main` generation and no hit-rate gain. Which paths go unread also depends on what a change touches. Evidence that would change the call: a `main` generation over 5 GB, which leaves room for only one.
  - **`cache-mode: read` on pull request jobs.** This duplicates the `if:` guard, and the export step (22-72 s) would still run. Evidence that would change the call: GitHub dropping support for step-level skips.
  - **Delete superseded `main` generations.** Eviction already deletes the entry with the oldest access date first, which is the older generation, and deleting needs `actions: write` on the job. Evidence that would change the call: a newest `main` generation observed evicted.
- Not touched: the restore step and its key, the restore-key prefix, `Load the cached store paths`, `mutants.yml` (which uses no Actions cache), and every existing plan.

### Outstanding Questions

None blocks U1.

- Q1. **Merge order with #15.** Either order merges clean (Planning Contract, Merge with open PRs). Recommendation: merge whichever PR is green first. If #15 lands first, its `main` push seeds the first bundle, and this PR's next run is V3 with no extra step.
- Q2. **The source of V3 if this lands first.** #15 and #16 restore their own legacy entries (AE4), and PR7 is on hold. Recommendation: the operator deletes #16's two entries, IDs 8799211987 and 8799099032 (2.55 GB, `gh cache delete` needs `actions: write`, which the agent's PAT lacks), then closes and reopens #16. Its next run restores `main`'s bundle. #16's own store is `main` plus one test file, so it loses nothing.
- Q3. **R14's warm-run definition in the standalone plan** ("a re-run attempt of the same run, after the first attempt saved its caches") no longer applies to pull requests, which stop saving. Recommendation: leave R14 as the record of PR2's measurement. KTD4 defines cold and warm for this lane from here on.
- Q4. **#15's legacy entries (8 GB).** Recommendation: keep them. They are #15's warmest store, its runs keep accessing the newest pair, and eviction removes the older pair at the first `main` save. Once #15 merges, nothing accesses them and eviction removes them.

### Sources

- GitHub, [Dependency caching reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching): pull request caches are scoped to `refs/pull/.../merge`, the search covers the current ref and then the default branch, a prefix match returns the most recently created entry, eviction runs in order of oldest access date, and `pull_request` runs keep write access to their own merge ref.

---

## Planning Contract

### Key Technical Decisions

- KTD1. **A `github.ref == 'refs/heads/main'` condition on `Export the unsigned store paths` and `Save the Nix store cache`, joined to the existing `cache-hit != 'true'` test.** The ref test rather than `github.event_name == 'push'` lets a `workflow_dispatch` on `main` reseed the bundle. A dispatch on any other branch saves nothing. Skipping the export, and not only the save, removes 35-84 s per x86_64 pull request job and 26-51 s per aarch64 job. Governs R3, R4, R7.
- KTD2. **The restore step stays as it is.** `restore-keys: nix-store-${{ matrix.system }}-` already reaches `main`. GitHub searches the current ref, then the default branch, and returns the most recently created prefix match. Governs R1, R2.
- KTD3. **Footprint is bounded by eviction, not by a deletion step.** One generation is 2.55 GB today (x86_64 1.615 + aarch64 0.930) and 4.00 GB once #15 merges (2.357 + 1.643). Only `main` writes, so the steady state is 3 generations (7.6 GB) before #15 and 2 (8.0 GB) after. When a save pushes the total over 10 GB, GitHub keeps the new entry and evicts in order of oldest access date. That is an older `main` generation, or a legacy pull request entry nobody has touched since. The newest `main` generation was restored at the start of the run that saves its successor, so it is never the oldest. Governs R4.
- KTD4. **Cold and warm come from first runs, not re-runs.** The PAT cannot re-run or dispatch workflows, and a pull request re-run would restore nothing new because pull requests no longer save. A cold run is one with nothing to restore. A warm run restores `main`'s bundle by prefix. Governs R5.

### High-Level Technical Design

Restore and save by run kind once this change is on `main`:

| Run | Own ref holds a bundle | `main` holds a bundle | Restores | Exports and saves |
| --- | --- | --- | --- | --- |
| pull request | no | no | nothing (cold) | no |
| pull request | no | yes | `main`'s newest, by prefix | no |
| pull request (#15, #16 legacy) | yes | either | its own newest | no |
| push to `main`, key misses | (is `main`) | no | nothing (cold) | yes |
| push to `main`, key misses | (is `main`) | yes | previous generation, by prefix | yes |
| push to `main`, exact hit | (is `main`) | yes | exact entry | no (as today) |
| `workflow_dispatch`, other branch | no | either | as a pull request | no |

Estimated cost, from the jobs API numbers above. These are estimates, to be replaced by V1 to V4:

| Option | Footprint at 2 open PRs | PR first-run hit | Tests-only PR, x86_64 job |
| --- | --- | --- | --- |
| Today (save on every ref) | 12.0 GB after #15, so `main` is evicted | 0% while `main` is evicted | 592 s (measured, #16) |
| Delete superseded PR entry | 12.0 GB, so `main` is evicted | 0% while `main` is evicted | about 590 s on the first run |
| Narrow the export, per-ref saves | 7.7 GB, 10.2 GB at 3 open PRs | 0% once `main` is evicted | about 590 s on the first run |
| **Save on `main` only (KTD1)** | **8.0 GB, all `main`** | **100% by prefix once `main` has saved** | **about 190 s (237 s warm main minus 44 s export and save)** |

### Merge with open PRs

#15 (`feat/npm-tarball-release`, `0dfe7947`) adds steps between `Benchmark against PR1` and the export, and new jobs at the end. It does not touch the export or save steps. A scratch `git merge-tree` of the two-line `if:` change against `0dfe7947` merged clean in both orders to the same tree, with the conditioned steps after #15's `upload-artifact` step.

- This merges first: #15 then merges `main` with no conflict. Its runs restore its own legacy entries (AE4) and no longer save.
- #15 merges first: its `main` push still saves under today's rule, which seeds the first `main` bundle. This branch then merges `main` with no conflict, and its own next run restores that bundle.

#16 touches only `crates/gritlint_core/tests/rule_loading.rs` and its plan, so it shares no file with this change. Any push to `main` before this merges, #16's included, seeds a `main` bundle under today's rule.

### Assumptions

- GitHub's documented scoping, search order, prefix match and eviction rule (Sources) hold as documented on 2026-10-11.
- After #15 merges, a `main` generation stays near 4.0 GB, the size of #15's own entries.

---

## Implementation Units

### U1. Save the store bundle on main only

- **Goal:** pull request runs restore and never export or save. `main` runs behave as today.
- **Requirements:** R1 to R4, R6, R7. KTD1, KTD2.
- **Dependencies:** none.
- **Files:** `.github/workflows/ci.yml`, the `if:` of `Export the unsigned store paths` and of `Save the Nix store cache`.
- **Approach:** add the KTD1 ref test to both existing `if:` expressions. Nothing else changes: no action pin, `timeout-minutes`, key, restore-key or other step.
- **Patterns to follow:** the existing `matrix.system == 'x86_64-linux'` step conditions in `ci.yml`.
- **Test scenarios:** Test expectation: none. This is workflow configuration, and the Verification Contract runs prove it on GitHub.
- **Verification:** V1 to V5 below.

---

## Verification Contract

All times are per-job `completed_at - started_at` from `GET /repos/systemfsoftware/gritlint/actions/runs/{id}/jobs`, plus the `Nix store cache`, build, export and save step times. Cache facts come from `GET /repos/systemfsoftware/gritlint/actions/caches`.

| ID | Run | Proof | Fallback |
| --- | --- | --- | --- |
| V1 | Cold pull request: this PR's first run while `main` holds no bundle | `Nix store cache` restores nothing. Export and save are `skipped` on both systems. Wall time per system. | If a push to `main` came first, this run is V3. The cold pull request figure is then the pre-change run 38149555075 (x86_64 592 s, aarch64 282 s). |
| V2 | Cold push to `main`: the first push after merge while `main` holds no bundle | Nothing restored. Export and save run. The caches API shows one entry per system with ref `refs/heads/main`. | If `main` already held a bundle, the cold figure is run 38069169248 (x86_64 565 s, aarch64 261 s), and this push is V4. |
| V3 | Warm pull request: a run, after `main` holds a bundle, of a pull request whose own ref holds no entry | The restored key in the `Nix store cache` log equals a `refs/heads/main` key in the caches API. Export and save are `skipped`. Wall time per system. | Source per merge order: Q1, Q2. |
| V4 | Warm push to `main`: a push that restores a previous `main` bundle | Restores by prefix and saves a new generation. Wall time per system. | none |
| V5 | Footprint after V4 | Every entry created after the merge has ref `refs/heads/main`. No `refs/pull/*` entry is newer than the merge. | none |

Each job's time is listed, and a job over 10 minutes is named with its time. Nothing is cut.

---

## Definition of Done

- U1 merged: `.github/workflows/ci.yml` changed, nothing else.
- The PR body carries V1, and V3 when a push to `main` preceded the merge. A post-merge update carries V2 to V5, each with its run link and per-system wall times.
- No scratch worktree, probe branch or throwaway script remains.
