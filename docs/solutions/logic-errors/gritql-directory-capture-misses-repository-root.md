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

Single-file rules had the same defect without a capture. `tsconfig-condition`, `tsconfig-preset-module-mode`, `tsdown-exports`, `vitest-conditions` and `shared-vitest-config` named their files with `r".*/vitest\.config\.(?:ts|mts)"` and similar patterns, which also require a slash. They now use `r"(?:.*/)?..."`, and each has a root-level bad case. Two rules also check a parameter (`presetFiles`, `sharedVitestConfigFile`). The values this repository ships and documents (`gritlint.json`, the pack README, the bench config) moved to the same form, and the root cases use them verbatim, so the documented value is the one under test. The group must be non-capturing: a pattern that binds no variable and has a capture group fails every file with `regex pattern matched 1 variables, but expected 0`.

## Why this works

A root `tsconfig.json` binds `$dir` to `""`. The join holds only if the `package.json` clause can also bind `""`, so both captures must allow the empty alternative. Widening only one side still fails at the root.

## Invariants

- A directory capture used in a multifile join matches the empty directory: `(.*/|)`, never `(.*/)`.
- A file-name pattern that binds nothing matches the repository root: `(?:.*/)?name`, never `.*/name`.
- Every such rule has at least one case whose files sit at the case root, so the root path is exercised.
- Every join also has a case that pairs a root file with a nested one, judged against the rule's join conjunct. The empty capture must not join them: `""` is a directory like any other, not a prefix of every directory. In `internals-export-map`, a root `tsdown.config.ts` must not count as a nested package's bundler (`bad/root-bundler-nested-internals`). In `typecheck-build-mode`, a reference-only root `tsconfig.json` must not judge a nested `package.json` (`good/root-tsconfig-nested-package`).
- GritQL regexes match the whole path, so dropping the slash does not let `mytsconfig.json` match `tsconfig.json`.

## Prevention

Find rules and documented parameter values whose path pattern still requires a slash (none remain). The grep catches `.*/x`, `(.*/)x` and `(?:.*/)x`, and passes `(.*/|)` and `(?:.*/)?`:

```sh
git grep -n -P 'r\\?"(?:\((?:\?:)?)?\.\*/(?!\|\)|\)\?)' -- 'packs/*/rules/*.md' 'packs/*/README.md' gritlint.json crates/gritlint_bench/effect.gritlint.json
```
