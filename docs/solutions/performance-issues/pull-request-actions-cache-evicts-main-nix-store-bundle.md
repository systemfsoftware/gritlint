---
title: Pull request runs that save the Actions cache evict main's Nix store bundle, so every pull request builds cold
date: 2026-10-12
category: performance-issues
module: gritlint
problem_type: performance_issue
component: tooling
symptoms:
  - "A tests-only pull request (#16) restored no store bundle and its x86_64 job took 592 s against a 10-minute budget"
  - "The caches API held 10.5 GB in 6 entries, all refs/pull/15/merge and refs/pull/16/merge, and none for refs/heads/main"
  - "A push to main whose key equalled a pull request's saved key still logged 'Cache not found' and rebuilt cold"
root_cause: config_error
resolution_type: config_change
severity: high
related_components:
  - .github/workflows/ci.yml
tags: [github-actions, actions-cache, ci-cache, cache-eviction, nix, binary-cache, pull-request]
---

# Pull request runs that save the Actions cache evict main's Nix store bundle, so every pull request builds cold

## Problem

`ci.yml` exported the job's unsigned Nix store paths and saved them with `actions/cache/save` on every run whose key missed. One generation is about 4 GB (2.36 GB x86_64 plus 1.64 GB aarch64). GitHub creates a pull request's cache on `refs/pull/N/merge`, where only re-runs of that same pull request can restore it, and a branch's cache on that branch, yet both count against the repository's 10 GB limit. The two pull requests open on 2026-10-11 filled the cache, eviction removed `main`'s bundle, and from then on every pull request's first run restored nothing. Pull requests are not the only writers: the release flow dispatches `ci.yml` on `changeset-release/main`, and that run saved a full generation on its own branch too.

## Symptoms

- #16, a tests-only change, ran its x86_64 job in 592 s. Its step times from the jobs API: `Nix store cache` 0 s, build 486 s, export 35 s, save 9 s.
- On 2026-10-11 the caches API listed 6 entries totalling 10,545,221,338 bytes, all on `refs/pull/15/merge` and `refs/pull/16/merge` (the store-cache plan's Problem Frame).
- After #15 merged, the first `main` push computed `nix-store-aarch64-linux-f54d03c9…`. #15's pull request had saved that exact key (cache entry 8798771673, since evicted), and the push still logged `Cache not found`, because `main` cannot read a pull request's merge ref. Its x86_64 job took 893 s.
- On 2026-10-12 the release flow's `workflow_dispatch` on `changeset-release/main` (run 38266314981) saved 3,999,377,369 bytes on `refs/heads/changeset-release/main`, and #19's pull request saved 3,978,528,473 bytes on `refs/pull/19/merge`, taking the cache to 11,978,096,548 bytes. Between 16:45Z and 17:05Z #20 saved a pair on `refs/pull/20/merge` and #19 saved another aarch64 entry; by then the caches API listed 5 entries, all on those two merge refs, and none on `refs/heads/main`.

## What Didn't Work

These alternatives were costed while planning (`docs/plans/2026-10-12-1640-ci-store-cache-default-branch-only-plan.md`, Scope Boundaries) and rejected:

- **Deleting a pull request's superseded entry on each push.** One pair per open pull request still remains. With the two pull requests open on 2026-10-11, that plus `main` comes to about 12 GB, so `main` is evicted again. It also needs `actions: write`, which a fork pull request's token never gets.
- **Narrowing the export to the paths a rebuild reads.** That cuts a generation to about 2.5 GB, but per-ref entries stay unrestorable by any other ref, and `main` plus 3 open pull requests reach the 10 GB limit.
- **`cache-mode: read` on the job.** It blocks the save, but the export step, 22-74 s per job, still runs for nothing.

## Solution

Only the default branch exports and saves. Every run keeps restoring by prefix, and that search falls back to the default branch. In `ci.yml`:

```yaml
      - name: Nix store cache
        id: store-cache
        uses: actions/cache/restore@55cc8345863c7cc4c66a329aec7e433d2d1c52a9
        with:
          path: ${{ env.STORE_CACHE }}
          key: ${{ steps.targets.outputs.key }}
          restore-keys: nix-store-${{ matrix.system }}-
      # ...
      - name: Export the unsigned store paths
        if: github.ref == format('refs/heads/{0}', github.event.repository.default_branch) && steps.store-cache.outputs.cache-hit != 'true'
      # ...
      - name: Save the Nix store cache
        if: github.ref == format('refs/heads/{0}', github.event.repository.default_branch) && steps.store-cache.outputs.cache-hit != 'true'
```

The condition tests `github.ref`, not `github.event_name == 'push'`, so a `workflow_dispatch` on `main` can reseed the bundle while a dispatch on any other branch saves nothing. It reads the default branch from the event payload instead of naming `main`, because the restore fallback already follows the default branch by what it is: a literal would be a second copy of that fact, and a rename that updates the `push` trigger but misses the literal stops every save while every job stays green.

Measured from the jobs API (`completed_at - started_at` of each `Nix · <system>` job). #17, the pull request that made this change, computed the same keys as #16's merge commit, so its run is #16's case on a warm start:

| Run | x86_64 | aarch64 |
| --- | --- | --- |
| #16, pull request, before (restored nothing): run 38149555075 | 592 s | 282 s |
| #17, pull request, after (restored `main`'s previous generation by prefix; export and save skipped): run 38264981227 | 316 s | 182 s |
| `main` push of #15's merge, cold: run 38262818822 | 893 s | 439 s |
| `main` push of #16's merge, warm (restored, then saved a new generation): run 38262833748 | 404 s | 249 s |

## Why This Works

GitHub's [dependency caching reference](https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching) searches the current ref first and then the default branch, and a prefix match returns the most recently created entry. No ref other than the default branch can save under this rule, so `restore-keys: nix-store-<system>-` reaches the default branch's newest bundle. The exception is an entry another ref saved before the rule: that ref restores its own entry first and keeps it alive by accessing it, until it goes unaccessed for 7 days or is deleted. Once those are gone the cache holds default-branch generations alone. When a save takes the total over the limit, GitHub evicts in order of oldest last access. That is an older generation or a legacy entry, never the one just saved, nor the one the saving run restored at its start.

## Prevention

- Every step that saves an Actions cache (`actions/cache/save`, or `actions/cache`, which saves in its post step) is conditioned on the run's ref being the default branch, unless the entry is small enough that one per open pull request and per dispatched branch fits beside the default branch's. Gate: review. The reviewer runs `grep -nE -B1 -A1 'actions/cache(/save)?@' .github/workflows/*.yml` and confirms each save step's `if:` compares `github.ref` with the default branch.
  - wrong: `if: steps.store-cache.outputs.cache-hit != 'true'` on a save step.
  - right: `if: github.ref == format('refs/heads/{0}', github.event.repository.default_branch) && steps.store-cache.outputs.cache-hit != 'true'`.
- When CI slows down, list the cache entries by ref: `gh api 'repos/systemfsoftware/gritlint/actions/caches?per_page=100' --jq '.actions_caches[] | [.ref, .key, .size_in_bytes, .last_accessed_at] | @tsv'`. Entries on any ref other than `refs/heads/main` that are newer than the change mean a writer escaped the guard. Older ones are legacy entries; `gh cache delete <id>` removes them and needs `actions: write`.
- The bundle is not permanent. GitHub deletes an entry nobody has accessed for 7 days. A pull request restore counts as an access: the caches API moved `main`'s x86_64 entry's `last_accessed_at` to the minute a pull request job restored it. So this happens only after a week with no CI run at all. The next `main` push, or a `workflow_dispatch` on `main`, reseeds it, and until then pull requests build cold.

## Related Issues

- #16: the cold run that exposed this. #17: the fix, unmerged as of this writing.
- `docs/solutions/performance-issues/fetch-cargo-vendor-given-whole-source-rebuilds-crane-deps.md`: the same store bundle, and why a `file://` cache carries the full closure of its unsigned roots.
