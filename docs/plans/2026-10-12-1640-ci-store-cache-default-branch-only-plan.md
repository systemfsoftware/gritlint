---
title: Save the Nix store cache on the default branch only - Plan
type: ci
date: 2026-10-12
topic: store-cache-main-only
supersedes: docs/plans/2026-10-11-2237-ci-store-cache-main-only-plan.md
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# Save the Nix store cache on the default branch only - Plan

## Goal Capsule

- **Objective:** every pull request run and every push to `main` starts from `main`'s newest Nix store bundle, so a tests-only pull request finishes well inside the 10-minute PR budget instead of rebuilding the GritQL dependency tree cold.
- **Means:** only runs on the repository's default branch (`main`) export and save the bundle, and the save condition derives the default branch from the event, not from a literal name. Every other run restores and never saves (KTD1).
- **Authority:** this plan, then the sub-conductor and root rulings of 2026-10-11 and 2026-10-12. It supersedes `docs/plans/2026-10-11-2237-ci-store-cache-main-only-plan.md`, whose U1 shipped `github.ref == 'refs/heads/main'` and whose full-path review found that literal against CONST-T12 (KTD1). KTD1 replaces the saving sentence of KTD13 in `docs/plans/2026-10-11-1301-feat-gritlint-standalone-plan.md` (R7). That plan stays frozen.
- **Stop conditions:** stop and ask before adding any tool or action, changing an action pin, adding a credential, an external cache or a CI gate, or changing `timeout-minutes`.
- **Execution profile:** two units on `ci/store-cache-main-only`, cut from `origin/main` (`d96dc51c1`) with `origin/main` at `f552830c1` (#15, #16) merged in. U1 (`c62cda5f9`) is executed; U2 replaces its literal. The PR into `main` carries the Verification Contract numbers available before merge, and a post-merge update carries V5 and V6. The operator merges.

---

## Product Contract

### Summary

The two steps that export and save the Nix store bundle in `.github/workflows/ci.yml` run only when the run's ref is the repository's default branch. Every other run keeps the restore step, which falls back to the default branch through the `nix-store-<system>-` prefix, and skips the export and the save.

### Problem Frame

On 2026-10-11 the repository's Actions cache held 10,545,221,338 bytes in 6 entries, over GitHub's 10 GB limit. All 6 belonged to `refs/pull/15/merge` and `refs/pull/16/merge`. `main` had none, because they had been evicted. `ci.yml` saves a full store export from every run whose key misses, and the key hashes every output path, so nearly every push misses. GitHub scopes a pull request's cache to its merge ref, which only re-runs of that same pull request can restore, and a branch's cache to that branch. Every pull request, and every `workflow_dispatch` on another branch, therefore pays up to 4 GB for an entry no other run can use, and evicts the one bundle every run could use.

Result: #16, a tests-only pull request, restored nothing. Its x86_64 job took 592 s (run 38149555075: `Nix store cache` 0 s, build 486 s, export 35 s, save 9 s), against the 10-minute PR budget.

The writers are not only pull requests. On 2026-10-12 the release flow dispatched `ci.yml` on `changeset-release/main` (run 38266314981), which saved 2,356,808,121 + 1,642,569,248 bytes on `refs/heads/changeset-release/main`. At 16:2xZ the cache held 11,978,096,548 bytes: the newest `main` generation, an older `main` aarch64 entry, one `changeset-release/main` entry, and #19's fresh pair on `refs/pull/19/merge`.

### Requirements

**Restore**

- R1. A run whose own ref holds no store bundle restores the default branch's newest bundle for its system, whenever one exists.
- R2. A push to the default branch restores its previous bundle for its system, whenever one exists.

**Save**

- R3. Only runs on the default branch export and save a store bundle. A pull request run, or a run on any other branch, neither exports nor saves.
- R4. After the change merges, every new cache entry has the default branch's ref. No other ref adds to the cache footprint.
- R8. The save condition identifies the default branch from what it is (`github.event.repository.default_branch`), not from its name, so a default-branch rename cannot leave the save condition behind (CONST-T12).

**Measurement**

- R5. CI cost is reported from the jobs API as per-job wall time on x86_64 and aarch64, for 4 runs: a cold and a warm pull request run, where the warm run restores the default branch's bundle, and a cold and a warm push to the default branch. The budgets are measured, not enforced. `timeout-minutes` stays at 60, and a job over 10 minutes is listed with its time, not cut.

**Constraints**

- R6. No new tool or action. Every action keeps its full-SHA pin. No credentials, no external cache, no new CI gate.
- R7. KTD1 replaces the sentence of the standalone plan's KTD13 that begins "Saving happens on every ref". The standalone plan is not edited, because it is the contract its executed units were verified against.

### Acceptance Examples

- AE1. **Covers R1, R3.** Given `main` holds bundle `nix-store-x86_64-linux-<h1>` and a pull request ref holds none, when the pull request's x86_64 job runs with key `<h2>`, then `Nix store cache` restores `<h1>` by prefix, the build runs, and `Export the unsigned store paths` and `Save the Nix store cache` are skipped.
- AE2. **Covers R2, R3, R8.** Given `main` holds `<h1>`, when a push to `main` produces key `<h2>`, then the job restores `<h1>`, builds, and saves `<h2>` with ref `refs/heads/main`.
- AE3. **Covers R3.** Given a push to `main` whose key exactly matches an existing `main` entry, export and save are skipped, as before.
- AE4. **Covers R1.** Given a ref that still holds an entry saved before this change (a pull request's merge ref, or `changeset-release/main`), its run restores its own entry first, because GitHub searches the current ref before the default branch, and saves nothing. The entry ages out once nothing accesses it for 7 days, or the operator deletes it (Q5).
- AE5. **Covers R3, R4.** Given the release flow dispatches `ci.yml` on `changeset-release/main`, the run restores the default branch's bundle (or its own legacy entry, AE4) and skips export and save.

### Key Decisions

- **Save only on the default branch.** The alternatives under Scope Boundaries each leave entries on refs no other run can restore. Governs R3, R4.

### Scope Boundaries

- Considered and not built:
  - **Delete a pull request's superseded entry on each push.** One entry pair stays per open pull request: (open PRs + 1) x 4.0 GB, which is 12.0 GB with 2 open PRs, so `main` is evicted again. It needs `actions: write` on pull request runs, and fork pull requests never get that.
  - **Narrow the export to the paths a rebuild reads.** Measured against `0dfe7947` on x86_64: 271 exported paths, 5.50 GB of NAR, of which 1.98 GB (36%) a tests-only rebuild never reads. Kept per ref, the bundles still can't be restored by another ref, and main plus 3 open PRs reaches the 10 GB limit. Evidence that would change the call: a `main` generation over 5 GB, which leaves room for only one.
  - **`cache-mode: read` on non-default jobs.** It duplicates the `if:` guard, and the export step (22-74 s) would still run.
  - **A `schedule:` refresh run.** It keeps the bundle accessed through a quiet week, but it is a separate sweep, outside this change. The 7-day deletion stays an accepted risk: it needs a week with no CI run at all, and the next default-branch push or dispatch reseeds.
  - **A post-save assertion that a default-branch entry was written.** It is a new CI gate and needs `actions: read`; it waits for operator approval (GATE1). Detection today is the caches-by-ref query in the solution doc.
  - **An executable pin of GitHub's cache semantics** (boundary-testing/pin-dependency-semantics). That rule pins lockfile-versioned libraries behind boundary adapters with a test that re-fires on upgrade. GitHub's cache service is server-side and unversioned, nothing in a lockfile moves when it changes, and a change shows as a slower or cold job, which every PR measures, not as a silent correctness regression. The jobs-API wall times and the restore-key lines in each job log are the observation.
- Not touched: the restore step and its key, the restore-key prefix, `Load the cached store paths`, `on:` triggers and concurrency, `mutants.yml` (which uses no Actions cache), and every other plan.

### Outstanding Questions

None blocks U2.

- Q1. *(Resolved: #15 merged as `5e296917c`, then #16 as `f552830c1`.)*
- Q3. **R14's warm-run definition in the standalone plan** no longer applies to pull requests, which stop saving. Recommendation: leave R14 as the record of PR2's measurement; KTD4 defines cold and warm for this lane.
- Q5. **Legacy entries on other refs.** After merge, entries saved before the change on `changeset-release/main` and on open pull requests (#19) stay restorable by their own ref and keep counting against the limit while those refs keep accessing them. Recommendation: the operator deletes them once after merge (`gh cache delete <id>` needs `actions: write`, which the agent's token lacks). No new ones can appear.

### Sources

- GitHub, [Dependency caching reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching): pull request caches are scoped to `refs/pull/.../merge`, the search covers the current ref and then the default branch, a prefix match returns the most recently created entry, eviction runs in order of oldest access date, entries not accessed for 7 days are deleted, and `pull_request` runs keep write access to their own merge ref.
- GitHub, [Webhook events and payloads](https://docs.github.com/en/webhooks/webhook-events-and-payloads): `repository` is a required property of the payloads, and the repository object carries `default_branch`.

---

## Planning Contract

### Key Technical Decisions

- KTD1. **`github.ref == format('refs/heads/{0}', github.event.repository.default_branch)` on `Export the unsigned store paths` and `Save the Nix store cache`, joined to the existing `cache-hit != 'true'` test.** The ref test rather than `github.event_name == 'push'` lets a `workflow_dispatch` on the default branch reseed the bundle; a dispatch on any other branch saves nothing. The default branch comes from the event payload, not a literal, because the restore fallback already follows the default branch by what it is; a literal is a second copy of that fact which a rename can leave behind with every job still green (CONST-T12). Skipping the export, and not only the save, removes 35-84 s per x86_64 pull request job and 26-51 s per aarch64 job. Governs R3, R4, R7, R8.
- KTD2. **The restore step stays as it is.** `restore-keys: nix-store-${{ matrix.system }}-` already reaches the default branch. GitHub searches the current ref, then the default branch, and returns the most recently created prefix match. Governs R1, R2.
- KTD3. **Footprint is bounded by eviction, not by a deletion step.** One generation is 4.00 GB (2.357 GB x86_64 + 1.643 GB aarch64), so the steady state is 2 default-branch generations (8.0 GB). Until Q5's legacy entries go, they sit beside those generations, and eviction removes whichever entry was accessed longest ago; a legacy ref that keeps restoring its own entry keeps it alive. The newest default-branch generation was restored at the start of the run that saves its successor, so it is never the oldest-accessed entry. Governs R4.
- KTD4. **Cold and warm come from first runs, not re-runs.** The token cannot re-run or dispatch workflows, and a pull request re-run would restore nothing new because pull requests no longer save. Governs R5.

### High-Level Technical Design

Restore and save by run kind once this change is on `main`:

| Run | Own ref holds a bundle | Default branch holds a bundle | Restores | Exports and saves |
| --- | --- | --- | --- | --- |
| pull request | no | no | nothing (cold) | no |
| pull request | no | yes | default branch's newest, by prefix | no |
| pull request or other branch, legacy entry (AE4) | yes | either | its own newest | no |
| push to default branch, key misses | (is default) | no | nothing (cold) | yes |
| push to default branch, key misses | (is default) | yes | previous generation, by prefix | yes |
| push to default branch, exact hit | (is default) | yes | exact entry | no |
| `workflow_dispatch`, other branch (release flow on `changeset-release/main`) | legacy only | either | as a pull request | no |
| `workflow_dispatch`, default branch, key misses | (is default) | either | as a push | yes |

### Assumptions

- GitHub's documented scoping, search order, prefix match, eviction and 7-day rules (Sources) hold as documented on 2026-10-12.
- A default-branch generation stays near 4.0 GB.

---

## Implementation Units

### U1. Save the store bundle on main only *(executed, `c62cda5f9`)*

- **Goal:** pull request runs restore and never export or save. `main` runs behave as before.
- **Files:** `.github/workflows/ci.yml`, the `if:` of `Export the unsigned store paths` and of `Save the Nix store cache`.
- **Verification:** V1, V3 (pull request side), measured on this PR.

### U2. Derive the default branch instead of naming it

- **Goal:** the save condition follows the default branch by what it is (R8).
- **Requirements:** R3, R4, R6, R8. KTD1.
- **Dependencies:** U1.
- **Files:** `.github/workflows/ci.yml`, the same two `if:` lines; `docs/solutions/performance-issues/pull-request-actions-cache-evicts-main-nix-store-bundle.md`, which states the condition and its Prevention rule.
- **Approach:** replace `'refs/heads/main'` with `format('refs/heads/{0}', github.event.repository.default_branch)` in both `if:` expressions. Nothing else in the workflow changes.
- **Test scenarios:** Test expectation: none. This is workflow configuration; no test layer admits it (no published surface, decision cell or regression input), and the Verification Contract runs prove it on GitHub.
- **Verification:** V3 on this PR's new head (the condition evaluates false on a pull request run, which GitHub reports as `skipped`), V5 and V6 after merge.

---

## Verification Contract

All times are per-job `completed_at - started_at` from `GET /repos/systemfsoftware/gritlint/actions/runs/{id}/jobs`, plus the `Nix store cache`, build, export and save step times. Cache facts come from `GET /repos/systemfsoftware/gritlint/actions/caches`, with ref, key, `size_in_bytes` and `last_accessed_at` recorded per entry.

| ID | Run | Proof | Source |
| --- | --- | --- | --- |
| V1 | Cold pull request | Nothing restored. Wall time per system. | Pre-change run 38149555075 (x86_64 592 s, aarch64 282 s); `main` already held a bundle when this PR opened. |
| V2 | Cold push to `main` | Nothing restored; export and save run; one `refs/heads/main` entry per system. | Run 38262818822 (#15's merge), old rule, same behaviour for `main`. |
| V3 | Warm pull request | The restored key in the `Nix store cache` log is a `refs/heads/main` key in the caches API; export and save `skipped`; no `refs/pull/17/merge` entry appears. | This PR's runs, on every head. |
| V4 | Warm push to `main` | Restores by prefix and saves a new generation. | Run 38262833748 (#16's merge), old rule. |
| V5 | First output-changing push to `main` after merge (the merge push itself is expected to be an exact hit) | `Save the Nix store cache` concludes `success`, not `skipped`, and the caches API shows a `refs/heads/main` entry for that run's `steps.targets.outputs.key` on each system. | Post-merge update. |
| V6 | Footprint after V5 | No entry on any ref other than `refs/heads/main` is newer than the merge; the total is under 10 GB once Q5's legacy entries are deleted or expired. | Post-merge update. |

Each job's time is listed, and a job over 10 minutes is named with its time. Nothing is cut.

---

## Definition of Done

- U1 and U2 merged: `.github/workflows/ci.yml` and the solution doc changed, nothing else outside `docs/plans/`.
- The PR body carries V1 to V4 and a full-path review receipt with a non-Claude cross-model pass. A post-merge update carries V5 and V6, each with its run link.
- No scratch worktree, probe branch or throwaway script remains.
