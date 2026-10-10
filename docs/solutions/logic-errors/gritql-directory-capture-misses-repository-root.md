---
title: A GritQL directory capture written (.*/) never matches a file at the repository root
date: 2026-10-10
category: logic-errors
module: gritlint
problem_type: logic_error
component: tooling
symptoms:
  - "typecheck-build-mode reported a reference-only tsconfig.json under packages/app but stayed clean when the same files sat at the repository root"
  - "gritlint check on a single-package repository lists a rule under zero-file rules although the same files nested one directory down produce a finding"
root_cause: logic_error
resolution_type: code_fix
severity: medium
related_components:
  - packs/typecheck-build-mode/rules/typecheck-build-mode.md
  - packs/source-resolution/rules
tags: [gritql, regex, multifile, bubble, repository-root, pack-rule]
---

# A GritQL directory capture written (.*/) never matches a file at the repository root

## Problem

Multifile pack rules join two files by directory. Each `file($name, $body)` clause captures its directory from `$name`, and the second clause compares its capture with the first one's. The `typecheck-build-mode` rule captured it with `r"(.*/)tsconfig\.json"($dir)`. The engine passes rules paths relative to the scan root, so a root-level `tsconfig.json` arrives as `tsconfig.json`, with no slash. In a single-package repository the rule was therefore silently clean: no finding and no error. `gritlint check` only listed it among the "zero-file rules".

## Failure mechanism

1. `(.*/)` requires at least one `/`. A root-level file's directory is the empty string, which the capture group cannot produce.
2. The `file` clause fails to bind, so the multifile join has nothing to compare and the rule matches no file.
3. Every fixture case put its files in a nested package directory. No case ever held a root-level file, so `gritlint test packs` stayed green over the gap.

## Solution

Make the capture accept the empty directory, on both sides of the join (PR5, pending merge):

```grit
$name <: r"(.*/|)tsconfig\.json"($dir),
...
$name <: r"(.*/|)package\.json"($manifest_dir),
$manifest_dir <: $dir,
```

The bad case `root-reference-only` keeps its files at the case root and asserts the `typecheck` script pair's range in the root `package.json`. It failed before the change and passes after it.

Five `source-resolution` rules carried the same capture: `api-extractor-tsconfig`, `export-map-order`, `internals-export-map`, `publish-config-exports` and `tsconfig-paths`. Each took the same change, with a root-level bad case that failed before it.

## Why this works

A root `tsconfig.json` binds `$dir` to `""`. The join holds only if the `package.json` clause can also bind `""`, so both captures must allow the empty alternative. Widening only one side still fails at the root.

## Invariants

- A directory capture used in a multifile join matches the empty directory: `(.*/|)`, never `(.*/)`.
- Every such join has at least one case whose files sit at the case root, so the empty-directory binding is exercised.
- Every such join also has a case that pairs a root file with a nested one. The empty capture must not join them: `""` is a directory like any other, not a prefix of every directory.

## Prevention

Find rules that use the slash-only form (none remain):

```sh
git grep -n -E 'r"\(\.\*/\)' -- 'packs/*/rules'
```
