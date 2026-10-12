---
title: Keying files by a GritQL TargetLanguage Debug string formats the grammar's node table for every file
date: 2026-10-10
category: performance-issues
module: gritlint
problem_type: performance_issue
component: tooling
symptoms:
  - "Language keys for the selected files cost about 0.2 s of the walk and selection on the PR3 corpus"
  - "The JSON language key was a 652-byte string holding the grammar's node table"
root_cause: wrong_api
resolution_type: code_fix
severity: medium
related_components:
  - crates/gritlint_core/src/domain.rs
tags: [gritql, target-language, debug, map-key, rust, performance]
---

# Keying files by a GritQL TargetLanguage Debug string formats the grammar's node table for every file

## Problem

gritlint batches selected files by a language key. PR1 built it with `format!("{engine:?}")` on GritQL's `TargetLanguage`. Each variant wraps a struct holding the grammar's node-type table, and the derived `Debug` prints all of it. On the PR3 corpus, the JSON key was 652 bytes and cost about 40 µs per file, 0.2 s of the walk and selection (plan KTD18, measured with a single-threaded profiling probe). Equal languages print equal strings, so nothing failed. It was only slow.

## Solution

Key by `grit_util::Language::language_name()`, a `&'static str` per grammar, in gritlint_core's `Language::of`:

```rust
use grit_util::Language as _;
key: engine.language_name(), // "JSON", "TypeScript", "TSX", ...
```

At the GritQL rev pinned in `Cargo.lock`, the `marzano-language` crate defines 24 `language_name` impls, all distinct, so batches group exactly as before. The PR3 benchmark's report differential stayed byte-identical.

## Why This Works

A derived `Debug` describes a whole value, so its cost grows with the value's fields. A grouping key needs identity only. A library-defined constant per variant gives identity at zero cost, as long as the constant is unique per variant, which is the property the second prevention check pins.

## Prevention

- Never key a map, cache or batch on `{:?}` of a third-party type. Use the identity the library exposes. Gate: review.
  wrong: `key: format!("{engine:?}")` / right: `key: engine.language_name()`
- When moving the GritQL pin, check that `language_name` stays unique, because two grammars sharing a name would merge their batches without any error. Gate: the script below exits 0 when the names are non-empty and unique. It exits 1 on a scratch copy with TSX renamed to `"TypeScript"`, and 1 when the source path is missing.

  ```sh
  V=$(nix build --no-link --print-out-paths .#packages.x86_64-linux.gritlint-unwrapped.vendor)
  names=$(grep -rh -A1 'fn language_name' "$V"/source-git-0/marzano-language-*/src | grep -o '"[^"]*"')
  test -n "$names" && test -z "$(printf '%s\n' "$names" | sort | uniq -d)"
  ```

## Related Issues

- `docs/plans/2026-10-11-1301-feat-gritlint-standalone-plan.md` R15, KTD18, U6
