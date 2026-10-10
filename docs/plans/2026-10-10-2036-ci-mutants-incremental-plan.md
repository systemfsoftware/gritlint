---
title: Incremental, sharded and resumable mutation runs on main - Plan
type: ci
date: 2026-10-10
topic: mutants-incremental
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
source_commit: systemfsoftware/gritlint@fe886c507
---

# Incremental, sharded and resumable mutation runs on main - Plan

## Goal Capsule

- **Objective:** every mutant a push to `main` changes is covered by a mutation report that finishes, and what a run has tested is never lost to a later push or a cancelled job. Pushes that arrive while a run is in flight collapse into the newest one, whose report covers them. Maintainers read caught, missed and timed-out mutants in one step summary.
- **Means:** one repo-local workflow, `.github/workflows/mutants.yml`, with a planning job that picks incremental (`--in-diff` from the last green `main` run) or full mode, a matrix of `--shard k/n` jobs sized from measured per-mutant times, and a summary job that merges the shards; tested results carry forward as artifacts for `--iterate` (KTD1 to KTD6).
- **Authority:** the root's unit request (this plan's R-IDs restate it), then the standalone plan's R21 and R27 (`docs/plans/2026-10-10-1309-feat-gritlint-standalone-plan.md`), then this plan's KTDs. This plan refines that plan's KTD30, which deferred sharding to a follow-up; it does not edit that plan.
- **Stop conditions:** stop and ask before adding any executable, action or crate not already pinned in this repository other than `actions/download-artifact` at a full SHA; before any local `cargo mutants` invocation other than `--list`; before any hand-applied mutant; before changing the runner size; if GitHub refuses the PR-branch dispatch (Q1); and if any job in the proof runs exceeds 10 minutes.
- **Execution profile:** one PR from `ci/mutants-incremental` into `main`, a single-layer `gh stack`, after the root gates this plan (two layers only under Q1's fallback). The PR changes `.github/workflows/mutants.yml` and `.cargo/mutants.toml` only. It carries no changeset intent, since nothing a consumer installs changes.

## Product Contract

### Summary

The mutants workflow tests only the mutants a push to `main` touched, measured from the last `main` SHA whose Mutants run succeeded, and falls back to a full run when there is none. Full runs are sharded, and each shard's mutation step is capped so no job can run past 10 minutes. A loop mutant costs at most about 34 seconds instead of 103. A later run at the same SHA skips what an earlier one caught, even if the earlier one was cancelled.

### Problem Frame

Run 38079438358 on `b3ee427` was cancelled after 61.5 minutes, because #10's merge at 20:22 started run 38083474823 and `cancel-in-progress: true` killed the first. In those 61.5 minutes it tested 373 of 471 mutants (201 caught, 47 missed, 2 timeouts, 123 unviable) and every result was discarded. The full run takes about 70 minutes on one `ubuntu-latest` job (KTD2), and each push to `main` starts it again from zero. The two timeouts, both `parse_markdown` loop mutants (`decode.rs:735`, `:743`), cost 3.6 s of build and 99 s of test each, because the automatic test timeout is 5x the 19.8 s baseline.

### Requirements

- R1. A push to `main` mutates only code changed since the last green `main` SHA: `cargo mutants --in-diff` over `git diff <green> HEAD -- '*.rs'`. The green SHA is the `head_sha` of the newest successful Mutants run on `main`, read from the GitHub API with read-only permissions. With no such run, or when that SHA is not an ancestor of `HEAD`, the push runs the full sharded run. A failed API read (non-2xx, or a body without `head_sha`) fails the plan job with a named error; it is not treated as "no green run".
- R2. A full run (`workflow_dispatch` with `mode: full`, and R1's fallback) is sharded across a matrix with `--shard k/n` on `ubuntu-latest`, and `n` comes from measured per-mutant times with the arithmetic stated (KTD2).
- R3. A cancelled or partial run does not lose what it tested: a running `main` run is never cancelled by a later push, and a later run at the same SHA, including after a cancellation, skips mutants an earlier one caught or found unviable (`--iterate`, KTD5). No mutant's test runs longer than `max(20 s, ceil(1.5 × the baseline test time))` (KTD4).
- R4. Report-only stays: missed (exit 2) and timeout (exit 3) outcomes leave the run green and are listed in each shard's step summary and in a merged summary; any other exit code, including 1 (usage) and 4 (baseline failed), fails the run.
- R5. Proof runs on the PR branch through `workflow_dispatch`. Nothing records gate state: the green SHA is read from the API, never stored, and carried-forward results are artifacts of the runs that produced them. The PR body carries cold and warm wall times per shard from the jobs API.
- R6. No local mutation: no `cargo mutants` run locally beyond `--list`, and no hand-applied mutant.
- R7. Every job in the workflow finishes within 10 minutes including setup; for shards this holds by construction (KTD2). No pull-request job changes: R27 of the standalone plan stays as it is, and the workflow still never runs on `pull_request`.

### Acceptance Examples

- AE1. **Covers R1.** Given an incremental run whose base is `b3ee427` and whose head is the PR branch (no `.rs` change of its own), when it plans, then it lists the 10 mutants `cargo mutants --list --in-diff` lists over `b3ee427..fe886c5`, runs one shard, and tests no other mutant.
- AE2. **Covers R1.** Given no successful Mutants run on `main`, when a push runs, then the plan job's summary says `full (no green main run)` and the shards cover every listed mutant.
- AE3. **Covers R1.** Given a diff with no `.rs` change, when an incremental run plans, then it lists 0 mutants, no shard starts, and the run is green.
- AE4. **Covers R2, R7.** Given the full list (475 at `fe886c5`), when a full run plans, then `n = ceil(475 / 13) = 37`, and every job's wall time from the jobs API is under 10 minutes.
- AE5. **Covers R3.** Given a finished full dispatch at SHA S, when a second full dispatch at S runs, then its plan lists only the mutants the first did not catch or find unviable, and the shards test only those.
- AE6. **Covers R3.** Given a full dispatch at S cancelled while its shards run, when the next full dispatch at S plans, then its list excludes every mutant the cancelled shards caught or found unviable before they stopped.
- AE7. **Covers R4.** Given shards that report missed mutants, when the summary job runs, then the merged table's counts equal the sums of the shards' and the merged missed list is the union of theirs.

### Scope Boundaries

- No change to `ci.yml`, to any pull-request job, or to the runner size.
- No new test file and no new flake check: the workflow is proved by its own runs (KTD7).
- No attempt to make the test suite faster (the 19.8 s baseline dominates every missed mutant); no `--baseline=skip`.
- No scheduled full run; a full run is dispatched by hand or is R1's fallback.

### Dependencies / Assumptions

- cargo-mutants 27.1.0 from the locked nixpkgs, as now. Verified in its source at tag `v27.1.0` (`8ab1dc7`): `--in-diff`, `--shard k/n`, `--sharding slice|round-robin` (default `slice` since 26.0.0), `--iterate`, `--timeout-multiplier` and the `timeout_multiplier` config key, `--minimum-test-timeout` (default 20 s). `main.rs:578-611` applies `--iterate`, then `--in-diff`, then `--shard`, in that order. `in_diff.rs:47-57`: an empty diff, a diff with no Rust file, or one that touches no mutant exits 0; a diff that does not match the tree exits nonzero. `timeouts.rs:60-66`: test timeout = `max(minimum_test_timeout, ceil(baseline × multiplier))`. `output.rs:272-287`: `--iterate` skips the names in `mutants.out/caught.txt`, `unviable.txt` and `previously_caught.txt`; missed and timed-out mutants are tested again. `main.rs:617-619`: each run writes the whole loaded skip list to its own `previously_caught.txt`, so every shard repeats it.
- GitHub: with `cancel-in-progress: false` a group holds one running and at most one pending run, and a newer pending run replaces the older one (Actions docs, Concurrency). When a run is cancelled, a running job's later `always()` steps still run (Actions docs, Workflow cancellation reference: it re-evaluates `if` for running jobs and their unfinished steps). Whether a job that has not started yet runs on cancellation is not relied on (KTD5).
- `gh` is preinstalled on `ubuntu-latest` and is GitHub's own CLI; the plan job uses it for API reads.

### Outstanding Questions

- Q1. *(Resolve during ce-work, first push)* `main`'s copy of `mutants.yml` has no `workflow_dispatch` trigger. GitHub's manual-run guide says a dispatched workflow "must be in the default branch"; the file is there, but whether GitHub accepts a dispatch of the PR branch's copy is not settled by the docs. ce-work tests it with the first dispatch. If GitHub refuses, the fallback is a bottom stack layer that adds only `workflow_dispatch:` to `main`'s current file, which runs nothing until someone dispatches it, with the redesign as the layer above. Recommended: approve that fallback now, so a refusal does not stop the unit.
- Q2. *(Resolve Before ce-work)* Shard capacity is set so that a shard of 13 missed mutants with up to 4 timeouts still finishes its mutation step inside the cap (KTD2), which makes a full run 37 shards. Sizing from the mean (8.99 s per mutant) would give about 19 shards and half the setup minutes, but a shard of missed mutants would then hit its step cap and fail, so a full run's coverage would depend on how mutants fall. Recommended: 37.
- Q3. *(Resolve Before ce-work)* `--in-diff` sees only changed source, so a push that changes `.cargo/mutants.toml` (which mutants exist), `Cargo.lock` or only tests runs no new mutant. Recommended: keep R1 literal; a maintainer dispatches a full run after such a change. Alternative: the plan job switches to full mode when `.cargo/mutants.toml` changes.

## Planning Contract

### Key Technical Decisions

- KTD1. **One workflow, three job kinds.** Triggers: `push` to `main`, and `workflow_dispatch` with inputs `mode` (`full` or `incremental`, default `full`) and `base` (optional SHA, incremental only). For a push, mode is incremental with R1's green SHA as base, falling back to full as R1 says. For a dispatch the `mode` input decides; an incremental dispatch uses `base` if given, else the green SHA (with no green run it runs full, as a push does), and a base that is not an ancestor of `HEAD` fails the plan job by name. `plan` (one job): checkout with full history, the offline cargo home from the vendor tree (as now), gather the resume lists (KTD5), choose the mode, write `mutants.diff`, run `cargo mutants --workspace --list --iterate [--in-diff mutants.diff]`, count the lines, and output `mode`, `base`, `count`, `n` and the shard index list as JSON. It uploads `mutants.diff` and the resume lists as one artifact, `mutants-plan-<attempt>`, so every shard filters with the same bytes. Its step summary states mode, base, count and KTD2's arithmetic. `shard` (matrix over the index list, `if: needs.plan.outputs.count != '0'`, `fail-fast: false`): checkout `github.sha`, the offline cargo home, download `mutants-plan-<attempt>`, run `cargo mutants --workspace --iterate --shard k/n --sharding round-robin [--in-diff mutants.diff]`, write its own outcome table and missed list, and upload `mutants.out` as `mutants.out-<attempt>-<k>` in an `always()` step. `summary` (`if: always()`): download this attempt's `mutants.out-*`, write the merged table, missed list and timeout list; it does no state keeping. Governs R1 to R5.
- KTD2. **Shard capacity from measured times; the job bound holds by construction.** From run 38079438358's `outcomes.json` (373 mutants, `ubuntu-latest`): a missed mutant costs at most 24.7 s (3.6 s incremental build + the whole 19.8 s suite), a caught one 10.5 s on average, an unviable one 0.5 s, the mean is 8.99 s with KTD4's timeout applied, and a bounded timeout costs 33.6 s (8.9 s more than a miss). The baseline costs 57.6 s (37.8 s clean build + 19.8 s test), inside the `cargo mutants` step. The job reaches that step after 121 s (checkout, Nix, offline cargo home) plus about 5 s of artifact download; the summary table and upload after it take about 10 s. The step has `timeout-minutes: 7` (420 s), so a shard job lasts at most 126 + 420 + 10 = 556 s, under 600 s whatever its mutants do. Inside the step, mutants have 420 − 57.6 = 362.4 s. Capacity `C = 13`: 13 missed mutants take 321.1 s, which leaves room for 4 of them to be timeouts instead (356.7 s), and the fifth would hit the cap (365.6 s). `n = max(1, ceil(count / 13))`. A full run at `fe886c5` lists 475, so `n = 37`, and its expected shard time is 126 + 57.6 + 13 × 8.99 + 10 = 310 s. Incremental examples, from `--list --in-diff` at `fe886c5`: #10's diff (`b3ee427..fe886c5`) lists 10 mutants, one shard; `1d60be3..fe886c5` (#6 and #10) lists 161, 13 shards. A shard whose step hits the cap fails, and its tested mutants still upload (KTD5). The constant and these numbers live in a comment beside it. ce-work recomputes them from run 38083474823's `outcomes.json` if it has finished, and changes `C` only if its per-mutant maximum or baseline differs. Governs R2, R7.
- KTD3. **Round-robin sharding.** `--sharding round-robin` spreads each file's mutants across shards. Simulated on the measured durations, the slowest shard's mutant time under slice and round-robin is 177 s against 172 s at `n = 37`, but 566 s against 450 s at `n = 12`, the size of a large incremental run, because `domain.rs` and `decode.rs` hold 43 of the 47 missed mutants in consecutive runs. The 3.6 s median incremental build comes from the unsharded, source-ordered run; round-robin has less build locality, so the cold proof run's per-mutant build times are checked, and a median above 5 s sends this decision back. Governs R2.
- KTD4. **Bounded test timeout.** `.cargo/mutants.toml` gains `timeout_multiplier = 1.5`. On the measured runner the timeout becomes `max(20, ceil(19.8 × 1.5)) = 30` s, and a loop mutant costs 3.6 + 30 = 33.6 s instead of 102.7 s. The slowest non-timeout test in the run took 21.0 s, 1.06x the baseline, so 1.5x leaves 9 s of headroom. The multiplier follows each shard's own baseline, so a slower runner scales its timeout with it; below a 13.3 s baseline the 20 s floor binds instead. Governs R3.
- KTD5. **Resume through `--iterate` and artifacts at the same SHA.** The plan job reads the prior runs of this workflow at `github.sha` from the API (`actions/workflows/mutants.yml/runs?head_sha=<sha>`), takes the newest one before this run, and downloads its `mutants-plan-*` and `mutants.out-*` artifacts with `actions/download-artifact` (`run-id`, `github-token`); when this is a re-run attempt, it also downloads the earlier attempts' artifacts of this run. It unions their `caught.txt`, `unviable.txt` and `previously_caught.txt` with `sort -u` (every shard repeats the whole loaded list) into `mutants.out/` and uploads that inside `mutants-plan-<attempt>`. The shards' artifacts are written by `always()` steps of running jobs, so a cancelled run's shards still leave what they tested, and the plan artifact keeps the chain going even when no shard of a run started. Every shard filters with the plan's bytes, which matters because `--iterate` runs before `--shard`: shards that skipped different sets would slice different lists. Keying by SHA means a skip always refers to the same code and tests, so `--iterate`'s caution, that a later change can weaken coverage, does not apply. A re-run of only the failed shards repeats their original slice, because the plan job's outputs and artifact carry over; a re-run of all jobs resumes. Governs R3, R5.
- KTD6. **Queue on `main`, and fail only on errors.** `concurrency: { group: mutants-${{ github.ref }}, cancel-in-progress: false }`: a running `main` run finishes, and of the pushes behind it only the newest waits; the others collapse into it, and their mutants are covered because its diff starts at the last green SHA. Exit codes 0, 2 and 3 pass; every other code, and a step that hits KTD2's cap, fails the shard and the run (mutants.rs, Exit codes). Full mode keeps today's guard: a listed count of 0 fails, because it means `examine_globs` selects nothing; incremental mode accepts 0. Governs R3, R4, R7.
- KTD7. **No new test.** The workflow's behaviour is proved by its own dispatched runs (R5), and no permanent test would catch a consumer-visible defect that those runs do not: nothing outside CI consumes the workflow. Governs R5.
- KTD8. **Permissions and pins.** `contents: read` for every job; the plan job adds `actions: read` for the two API reads (`runs?branch=main&status=success&per_page=1` and `runs?head_sha=<sha>`) and for downloading another run's artifacts. New pin: `actions/download-artifact@9000827ccba6bdab643e8b6fd33ac0654aef8333` (v8.0.2). `actions/upload-artifact` keeps `043fb46d1a93c77aae656e7c1c64a875d1fc6a0a` (v7.0.1); both use the same artifact backend. No cache is used. Governs R1, R5.

### High-Level Technical Design

```text
push main / dispatch(mode, base)
  └─ plan: base = dispatch input | green SHA (API, push and incremental dispatch) → ancestor? → mode
           prior runs at github.sha (API) → their mutants-plan-* and mutants.out-* → sort -u → mutants.out/
           git diff <base> HEAD -- '*.rs' → mutants.diff
           cargo mutants --list --iterate [--in-diff] → count → n = max(1, ceil(count/13))
           upload mutants-plan-<attempt> (diff + resume lists)
  └─ shard[k] (k < n, only if count > 0): mutants-plan-<attempt>,
           cargo mutants --shard k/n --sharding round-robin --iterate [--in-diff]   (step cap 7 min)
           → step summary; always(): upload mutants.out-<attempt>-<k>
  └─ summary (always): merge counts, missed and timeout lists → step summary
```

### Risks

| Risk | Consequence | Handling |
| --- | --- | --- |
| A shard of missed mutants draws 5 or more timeouts | its step hits the 7-minute cap and the run fails | its tested mutants upload; a re-run of all jobs resumes them (KTD5); the PR states every shard's time |
| GitHub refuses the PR-branch dispatch (Q1) | no proof run before merge | the Q1 fallback layer, if approved; otherwise ce-work stops and reports it |
| The account's concurrent-job limit is below 37 | a full run's shards queue in waves | each job's own time is unchanged; the PR reports the total run time |
| A runner much slower than measured | the 13-mutant bound tightens | the timeout scales with the baseline (KTD4); the proof runs' times are the check, and a step that hits its cap stops ce-work |
| The push path itself (event, `main` concurrency group) is first exercised after merge | a defect there shows on the first push | the dispatches exercise the same plan job, green-SHA read and diff path; the first `main` run after merge is watched and its summary cited in the PR |

## Implementation Units

### U1. Sharded, incremental, resumable mutants workflow

- **Goal:** R1 to R7.
- **Files:** `.github/workflows/mutants.yml`, `.cargo/mutants.toml`.
- **Approach:** in this order. (1) `.cargo/mutants.toml` gains `timeout_multiplier = 1.5` (KTD4). (2) The plan job (KTD1, KTD2, KTD8), including mode selection, the API reads with named failures, the resume union and the plan artifact. (3) The shard matrix (KTD1, KTD3, KTD6) with per-shard summaries. (4) The summary job. (5) `cargo mutants --list` locally, with and without `--in-diff` over `b3ee427..fe886c5`, to check the counts the plan job will compute; nothing else runs locally (R6).
- **Test scenarios:** the dispatches run one after another, each started only after the previous run completed, because they share one concurrency group and a pending dispatch would replace an earlier pending one. Each run's ID goes in the PR body.
  - AE4 and AE7: a cold full dispatch (`mode: full`), the first run at the PR head.
  - AE6: a full dispatch at the same SHA, cancelled by hand once shards have tested mutants; then the warm run below shows its exclusions.
  - AE5: a warm full dispatch at the same SHA; its plan count equals the cold run's count minus everything caught or unviable so far.
  - AE1: an incremental dispatch with `base: b3ee427`; it lists the 10 mutants U1 step 5 listed locally.
  - AE3 or AE2: an incremental dispatch without `base`. Three outcomes are possible, and the PR body states which one ran: the green `main` SHA is the branch's base or later, so the diff has no `.rs` change (AE3); there is no green `main` run yet, so the plan reports `full (no green main run)` (AE2); or the green SHA predates `.rs` commits merged into the base, so the run lists their mutants, as in AE1. The third outcome is not an AE3 failure.
- **Verification:** every job's wall time from the jobs API; a table of cold and warm wall times per shard in the PR body; KTD3's build-time check from the cold run's `outcomes.json`; the plan job's arithmetic in its step summary; the YAML read in review, since no workflow linter is pinned here.

## Verification Contract

| Gate | Command or check | Proves |
| --- | --- | --- |
| Cold full run | `gh workflow run mutants.yml --ref ci/mutants-incremental -f mode=full`; jobs API wall times | R2, R4, R7, AE4, AE7 |
| Cancelled and warm full runs | the same dispatch, cancelled mid-shard, then again to completion; plan counts and shard times | R3, AE5, AE6 |
| Incremental path | `-f mode=incremental -f base=b3ee427`, then `-f mode=incremental`; plan summaries show base, mode and count | R1, AE1 to AE3 |
| Local listing | `cargo mutants --list` with and without `--in-diff` | R6, KTD2's counts, AE1's expected count |
| First `main` run after merge | its plan summary and jobs | R1 on the push path |

## Definition of Done

- One PR from `ci/mutants-incremental` into `main`, CI green on its head.
- The dispatched runs above finished in order, every job under 10 minutes, and the PR body carries the cold and warm per-shard wall-time table from the jobs API with each run's ID.
- Q1 to Q3 answered and folded in before ce-work.
