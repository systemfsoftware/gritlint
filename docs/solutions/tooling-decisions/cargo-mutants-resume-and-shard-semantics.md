---
title: Resuming and sharding cargo-mutants rests on its output rotation and filter order
date: "2026-10-10"
category: tooling-decisions
module: gritlint
problem_type: tooling_decision
component: tooling
severity: medium
applies_when:
  - Changing how the Mutants workflow seeds, shards or summarises a cargo-mutants run
  - Bumping cargo-mutants through the flake's nixpkgs input
  - Reviewing a claim that a shard's outcome files double-count carried mutants
related_components:
  - .github/workflows/mutants.yml
  - scripts/tools/mutants-plan.sh
  - .cargo/mutants.toml
tags:
  - cargo-mutants
  - mutation-testing
  - sharding
  - iterate
  - in-diff
retire_when: "cargo-mutants changes how --iterate reads mutants.out, how OutputDir::new rotates it, or the order it applies --iterate, --in-diff and --shard (check main and OutputDir::new at the pinned version)"
---

# Resuming and sharding cargo-mutants rests on its output rotation and filter order

## Problem frame

The Mutants workflow sizes its shard matrix in the plan job and resumes a cancelled run by seeding `mutants.out/` with what earlier runs at the same commit tested. Both designs are correct only because of two facts about cargo-mutants 27.1.0 that neither the workflow nor `mutants-plan.sh` shows. Without these facts, a reviewer can read the shard summary as double-counting carried mutants, and the plan job's count can look like the wrong input for the shard split. Both readings came up in review.

## Mechanism

1. **`--iterate` reads before it rotates.** In cargo-mutants `main`, the iterate branch calls `load_previously_caught` on `<parent>/mutants.out` and removes `caught.txt`, `unviable.txt` and `previously_caught.txt` from the discovered set. Only afterwards does `OutputDir::new` rename the existing directory to `mutants.out.old` and create a fresh one. `write_previously_caught` then records the loaded names in the new `previously_caught.txt`.
2. **The filters run in a fixed order: iterate, then `--in-diff` (`diff_filter_file`), then `--shard` (`Sharding::shard`).** `--list` returns at that point, before any output directory exists.

Consequences:

- Seeding `mutants.out/` before the run (the shard job copies the plan artifact's lists there) is how resume works.
- After a run, the four outcome lists hold only what that run tested, and the seed survives only as `previously_caught.txt`. The `summary` subcommand reads `previously_caught.txt` as "Carried", so nothing is double-counted.
- The `--list --iterate [--in-diff]` count is the exact set the shards split: $\bigcup_{k<n} \text{shard}(k/n) = \text{list}$, so $n = \lceil \text{count} / \text{SHARD\_CAPACITY} \rceil$ leaves no mutant unassigned.
- Exception: if cargo-mutants exits before `OutputDir::new` (for example on an argument error), the seeded lists stay in place unchanged. That shard step is red, with the error named.

`merge` folds `missed` and `timeout` into `previously_caught.txt` as well, so a warm or resumed run at the same commit skips them. A missed mutant is deterministic at a fixed commit. A timeout caused by runner load is not retried at that commit.

## Architectural invariants

- **Seed before, read the outcome lists after.** Never seed the outcome lists from carried data. Carried data belongs only in the input to `--iterate`; the outcome lists of a finished run hold only what it tested. Gate: `checks.<system>.gritlint-mutants-plan` (the `merge` and `summary` cases).
- **Size from the same filtered listing the shards split.** The count that sizes the matrix must come from `--list` with the same `--iterate`/`--in-diff` flags the shards use. Gate: the list-only probe below, re-run on every cargo-mutants bump.

## Verification

List-only shard-union probe, no mutant executed. At `fe886c5`, the union of 37 round-robin shards equalled the 475-mutant listing, and the union of 3 shards over the `b3ee427..fe886c5` diff equalled its 10 mutants.

```sh
for k in $(seq 0 $((n - 1))); do cargo mutants --workspace --list --shard "$k/$n" --sharding round-robin; done | sort | cmp - <(cargo mutants --workspace --list | sort)
```

On a cargo-mutants bump, also re-read the iterate/in-diff/shard block in `main` and `OutputDir::new` at the new version.
