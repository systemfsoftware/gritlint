---
title: Run the Mutants lane once a night on main - Plan
type: ci
date: 2026-10-13
topic: mutants-nightly
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
---

# Run the Mutants lane once a night on main - Plan

## Goal Capsule

- **Objective:** the Mutants lane stops running on every push to `main`. It runs once a night on `main` and on dispatch, refuses any other ref before it installs anything, never cancels or replaces another run, keeps every job near or under 10 minutes, and turns a red night into a fix task.
- **Means:** `.github/workflows/mutants.yml` takes a UTC `schedule` and `workflow_dispatch` with the `sha`, `packages` and `night` inputs. A `guard` job refuses off-`main` runs, and every other job needs it. Each run has its own concurrency group. The plan sizes shards from measured timings. With no triage workflow, the lane writes each night's record itself and files the fix task.
- **Authority:** `ci-wall-clock/mutation-runs-nightly-on-main` and the other packs this repository enrols at `de684e25` (#23), then this plan. Requirement R21 of `docs/plans/2026-10-12-2059-feat-gritlint-standalone-plan.md` ("runs in CI on pushes to `main` only") is superseded here on its trigger. Its other clauses stand: never on a pull request, no local mutant, counts and the missed list in the step summary. That plan stays frozen.
- **Stop conditions:** stop and ask before adding a third-party action or tool, a credential, a CI gate, or a `timeout-minutes` other than 60.
- **Execution profile:** one PR on `ci/mutants-nightly`, cut from `main` at `3919672c8`. Proof that needs the workflow on `main` (the first night, a dispatch, a refused branch dispatch) comes after merge, because this account's token cannot dispatch workflows (403 on 2026-10-12 and 2026-10-13).

---

## Product Contract

### Problem Frame

The lane ran on every push to `main` (`mutants.yml:3-6`), which the newly enrolled rule forbids. Its group keyed each push by commit and each dispatch by ref (`mutants.yml:18-20`). With GitHub's default `queue: single`, a third run in a group cancels the one already pending, so two dispatches on `main` could replace each other. A dispatch on any branch ran: six ran on `ci/mutants-incremental` and `proof/mutants-in-diff` between 2026-10-10 and 2026-10-11. Shards are sized by count (9 mutants each, `mutants-plan.sh:5`), so #20's run 38447766912 had 5 of 21 shards over 10 minutes (max 11m19s). Each shard spends 291 s on average (max 364 s) building the vendor tree and entering the dev shell before cargo-mutants starts.

### Requirements

- R1. Triggers are one daily UTC `schedule` and `workflow_dispatch`. There is no `push`, `pull_request` or `merge_group` trigger, and no other workflow calls or dispatches the lane.
- R2. A `guard` job runs first, after a `fetch-depth: 0` checkout and before anything installs. It fails a run whose ref is not the default branch, and it fails a `sha` that is not 40 hex digits or that the default branch's fetched tip does not reach (`git merge-base --is-ancestor --`). It also fails a `night` that is not a run id. Every other job lists `guard` in `needs:`.
- R3. Dispatch inputs `sha`, `packages` and `night`. The jobs check out `inputs.sha || github.sha`. `packages` empty plans every package. A set `packages` must be a non-empty JSON array of package names that `cargo mutants --list --json` reports, or the plan fails. `run-name` is `Mutants <sha>`, plus ` for <night>` when `night` is set.
- R4. Concurrency group `mutants-${{ github.run_id }}`, `cancel-in-progress: false`.
- R5. Night scope. The last green night is the newest `schedule` run on `main` that ended `success` and whose night record says green. A night tests the diff from that night's commit after checking it is an ancestor. It runs everything when no green night exists, when the commit is not an ancestor, or when the range changed the lane's own inputs: `.github/workflows/mutants.yml`, `.cargo/mutants.toml`, `scripts/tools/mutants-plan.sh`, `scripts/tools/mutants-night.sh`, or `flake.lock` (the cargo-mutants pin). A dispatch tests every mutant of its packages and resumes from earlier runs that tested the same commit. A dispatch with `night` set never resumes.
- R6. Shard sizing from measured timings. The plan reads the newest completed Mutants run on `main` that has `mutants.out-*` artifacts. It takes each source file's mean mutant time from `outcomes.json`, and the per-shard overhead from the jobs API (shard wall time minus its mutants' time). It then picks the fewest round-robin shards whose predicted wall time is at most 600 s. The step summary states the 10-minute target, the prediction, and each shard's measured wall time. The target is never a timeout.
- R7. Night record. A `schedule` run's `report` job uploads `mutants-night-<run_id>`, holding the commit, the base night and commit, the scope, outcome counts, survivors (missed), new survivors (missed and not in the base night's record), timeouts, failed jobs, completeness (every planned mutant has an outcome), the merges in the range with their pull requests, the verdict and the task number. Green means every job succeeded, the report is complete and there is no new survivor. A green night writes one summary line: run id, range, `green`.
- R8. Red night routing. A red night files one issue, the fix task. It names every merge in the range as a candidate (attribution ambiguous, since no triage re-tests), mentions each candidate pull request's opener (bots left out), and lists the survivors or failures with reproduce commands and predicted job-minutes. A survivor or failure that an open task already names updates that task with a comment and files nothing new. A night that no longer shows something an open task names says so on that task. The report job then ends red.
- R9. Every job sets `timeout-minutes: 60`, writes its wall time to the step summary, and on failure names a reason code from its closed set, the failing step and a reproduce command (`failures-name-the-gate-and-location`). The report job names every job whose result is `failure` or `cancelled`.
- R10. No job enters a dev shell. cargo-mutants, the pinned Rust toolchain and a C compiler come from the flake package `gritlint-mutants-tools`. Main's Nix store bundle is restored read-only before the tools are built.
- R11. The workflow header states runner-minutes per day for one night's run plus each dispatch, labelled as a model with the runs it comes from.

### Scope Boundaries

- No triage workflow (`workflow_run` handler, re-test bisection). The lane carries its own record and fix task (`mutation-runs-nightly-on-main`, Triggers clause). A triage is a later PR, and the `night` input is its re-test hook.
- No change to `ci.yml` and no new PR job.
- Killing #20's 10 missed mutants is F1, a separate PR. Until it lands, every night is red and runs everything.

## Key Technical Decisions

- KTD1. **Per-run group, not per-commit.** GitHub keeps one pending run per group by default and cancels it when a newer run queues (docs: "Control the concurrency of workflows and jobs", `queue: single`). A per-commit group would let a second dispatch at `main`'s tip replace a pending night or dispatch. `github.run_id` gives no main-side cancellation. Two runs at one commit may overlap, and resume reads only completed runs.
- KTD2. **Survivor means missed.** cargo-mutants' outcomes are caught, missed, unviable and timeout. Only missed means no test failed (mutants.rs, "Using the results"). A timeout means the suite hung until killed. The night records timeouts and reports them, but they do not turn a night red. Otherwise #20's 17 `lsp.rs` timeouts, which are server-loop mutants, would keep every night red.
- KTD3. **Round-robin prediction is exact.** `--shard k/n --sharding round-robin` gives shard k the mutants whose index i in `--list` order satisfies i mod n = k. A list-only probe at `3919672c8` showed shards 0 to 2 of 3 equal to `list[k::3]` over 627 mutants. The plan therefore predicts each shard's load from the same list the shards split.
- KTD4. **Timings from the newest measured run, with a labelled model to start.** Shard overhead uses the maximum over the source run's shards, so the old shape's 291-to-364 s setup sizes the first night conservatively and later nights correct themselves. With no source run, the plan uses a model from run 38447766912 (381 s overhead, 19 s per mutant) and says so in the summary.
- KTD5. **Store bundle restore.** Restore-only from `nix-store-x86_64-linux-` (main's bundle from `ci.yml`). It already holds the toolchain and vendor requisites, so a hit skips the 291 s setup. No step saves.
- KTD6. **Resume matches the tested commit.** A run resumes from completed runs whose title is `Mutants <tested sha>` (any suffix) or that were push runs at that commit. The run's `head_sha` is `main`'s tip, not `inputs.sha`.
- KTD7. **Decisions in tested scripts, I/O in the workflow.** `mutants-plan.sh` gains `guard`, `timings`, `shards` (from timings), `resume-runs` and `job-report`. The new `mutants-night.sh` holds `verdict` (green or red, new survivors, completeness) and `route` (which open task to update, what to file, what no longer shows). Each is a pure function of files and arguments. The workflow steps do the `gh` calls and pass their JSON in. Tests admitted (test-layer-selection, default refuse): one decision table per subcommand, covering its boundaries (the 40-hex and ancestor checks, a timing source present or absent, a budget one mutant cannot fit, a new survivor against a base survivor, a named item tested and gone against untested). Refused: tests of the `gh` loops and the YAML wiring (forwarding only). Those are proved by the first night (V6).
- KTD8. **Destructive review, Edge-First lens.** Three assumptions were checked. (1) The lane must own its record and task now: a dispatch-only stopgap, as `systemfsoftware`'s own lane has today, fails the rule's daily-`schedule` gate and the acceptance's first-night proof, so it is rejected. (2) Prior runs' artifacts stay readable: they last 90 days, and a miss only falls back to the model or to a full night, which over-tests and never under-tests. (3) A red night can always be routed: a failed plan, a cancelled shard or a backstop timeout still reaches `report` (`if: always()` after `guard` succeeds), and a night whose own run never starts is covered by the next night's range, since that range runs from the last green night. Rejected alternative: a separate `workflow_run` triage now. The rule then requires its re-test bisection, which is a later PR (Scope Boundaries).

## Implementation Units

### U1. Tools package without a dev shell
- **Files:** `flake.nix`.
- **Approach:** `packages.<system>.gritlint-mutants-tools`, a `buildEnv` of the rust-toolchain.toml toolchain, `cargo-mutants`, the stdenv C compiler and `jq`. The check runs both test files.
- **Verification:** `nix build .#packages.x86_64-linux.gritlint-mutants-tools`. `cargo mutants --list` through it gives the same 627 mutants as the dev shell.

### U2. Planner
- **Files:** `scripts/tools/mutants-plan.sh`, `scripts/tools/mutants-plan.test.sh`, fixtures.
- **Approach:** R2, R3, R5, R6, R9 decisions. Drop the `push` mode and the `mode`/`base` inputs.
- **Verification:** the test file, through `checks.x86_64-linux.gritlint-mutants-plan`.

### U3. Night record and routing
- **Files:** `scripts/tools/mutants-night.sh`, `scripts/tools/mutants-night.test.sh`, fixtures.
- **Approach:** R7, R8.
- **Verification:** the test file's decision tables (`verdict`, `route`) for green, red with a new survivor, red updating an open task, and no-longer-shows against untested.

### U4. Workflow
- **Files:** `.github/workflows/mutants.yml`.
- **Approach:** R1 to R4, R9 to R11.
- **Verification:** actionlint. After merge: the first night, a dispatch on `main`, and a dispatch on a branch that the guard refuses. Each is cited by run id in a follow-up comment on the PR.

### U5. Learning refresh
- **Files:** `docs/solutions/tooling-decisions/cargo-mutants-resume-and-shard-semantics.md`.
- **Approach:** replace the count-based sizing statement with the timing-based one, and add the round-robin index fact (KTD3).

## Verification Contract

| # | Check | When |
| --- | --- | --- |
| V1 | `bash scripts/tools/mutants-plan.test.sh` and `mutants-night.test.sh` pass; `checks.x86_64-linux.gritlint-mutants-plan` builds | before PR |
| V2 | actionlint on `mutants.yml` is clean | before PR |
| V3 | `cargo mutants --list` through `gritlint-mutants-tools` lists 627 mutants, the same as the dev shell (list-only) | before PR |
| V4 | The planner over run 38447766912's and 38130956835's artifacts predicts every shard of a full night at or under 600 s | before PR |
| V5 | CI on the PR is green, and the PR head shows no Mutants run | PR |
| V6 | The first `schedule` run after merge: title, `guard` success, plan summary, shard wall times, night record artifact, task or green line | after merge |
| V7 | A `workflow_dispatch` on `main` runs; one on a branch is refused by `guard` with `mutants-off-main` | after merge (operator dispatch) |
