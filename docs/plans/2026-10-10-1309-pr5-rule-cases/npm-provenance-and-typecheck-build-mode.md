# PR5 gap analysis — `npm-provenance` (3 rules) + `typecheck-build-mode`

Harness semantics read from `crates/gritlint_core/src/decide.rs`:

- `fixture_results` (493) fails a rule when `good` or `bad` is empty, and additionally fails when `locate` is declared but no bad case's `expected-locations.txt` line matches `:\d+:\d+-\d+:\d+$` (`asserts_a_range`, 485). Every one of the 4 rules has at least one range-asserting bad case, so `locate` is asserted for all of them.
- `evaluate` (549) recompiles the rule with `case.params`, plans over **only that case's files**, scans, and then: bad + Findings → `location_mismatch`; good + Clean → pass; bad + Clean → "produced no finding"; good + Findings → fail. **Consequence: a good case fails under any widening bug, and a bad case fails under any narrowing bug or any location change.**
- `location_mismatch` (593) compares the sorted list of `expected_location_line` strings, so **a changed range fails the case even when the finding itself is still correct**. One finding per case is asserted wherever the case lists one line.
- `plan`/`partition_multifile` (46, 65) splits a multifile rule's inputs **by parent directory**: one `DirectoryBatch` per directory, one task each. A file can only ever be matched together with siblings in its own directory.
- `place` (152) runs `locate` only on the **matched** files of the matched directory; it folds candidate `(path, range)` keeping the **first** match for a given path (`best.0 <= candidate.0`) and the **lexicographically smallest** path. With no locate match it falls back to `Location::new(matched.first(), None)` — i.e. a **file-only** finding.
- Range convention confirmed against committed lines: `bad/restricted` `10:5-10:27` for `    "access": "restricted",` — the property `"access": "restricted"` starts at col 5 and is 22 chars, so **end column is exclusive**. `bad/tsc-no-emit` `3:16-3:57` confirms it for the `typecheck` property.

---

## Rule 1 — `npm-provenance/provenance`

### Clauses
Match (`language json`, multifile `file($name, $body) where { … }`):
- **C1** `$name <: r".*package\.json"`
- **C2** `$program <: not contains "\"private\": true"`
- **C3** `$program <: contains "\"name\": $_"`
- **C4** or-branch A: `not contains "\"publishConfig\": $_"`
- **C5** or-branch B: `contains "\"publishConfig": $publish where { $publish <: not contains "\"provenance\": true" }`

Locate: `or { … , … }`
- **L1** `"provenance": $value where { $value <: not `true`, $value <: within `"publishConfig": $_` }`
- **L2** `"publishConfig": $publish where { $publish <: not contains `"provenance": $_` }`

### Existing cases
- `good/private-package` — `"private": true`, no name-driven requirements: pins C2.
- `good/provenance-true` — scoped, correct repo, `publishConfig.provenance: true`: pins C5 negation, C4 `or`→`and`, C3.
- `bad/no-publish-config` — public, no `publishConfig`: pins C4 branch A, C1, C2, C3; and the locate fallback (no L1/L2 match ⇒ file-only line).
- `bad/provenance-false` — `publishConfig.provenance: false` ⇒ `11:5-11:24`: pins C5's `not contains` and the **L1 vs L2 precedence** (L2 would give the `publishConfig` object `9:3-12:4`) and L1's node shape (key-only would give `11:5-11:16`).

### Coverage

| Clause | Plausible bug | Killing case | Verdict |
|---|---|---|---|
| C1 | `or`-branch/widening `.*package\.json` → `.*\.json`, or unescape `package.json` | none: every fixture file that is a package is named `package.json`; dropping the `\.` escape still matches it | **GAP** |
| C2 | `not contains` → `contains` | `good/provenance-true`, `good/private-package` | covered |
| C2 | literal widened to `"private": $_` | none: no case has a non-`true` `private` | **GAP** |
| C3 | conjunct dropped (report files without `"name"`) | none: every fixture `package.json` has a `name` | **GAP** |
| C3 | `"name": $_` → `"name"` | none | folded into the C3 gap |
| C4 | branch A dropped | `bad/no-publish-config` (stops reporting) | covered |
| C4 | `or` → `and` | `good/provenance-true` | covered |
| C5 | `not contains` → `contains` | `good/provenance-true` | covered |
| C5 | `or` → `and` | `good/provenance-true` | covered |
| C5 | literal widened `"provenance": true` → `"provenance"` / `"provenance": $_` | none: the only publishConfigs are `provenance: true` (clean) and `false` (reported by both) | **GAP** |
| C5 | `$publish` replaced by `$program` | no case separates them (`bad/no-publish-config` has no `provenance` anywhere) | **AMBIGUOUS** — not on the sanctioned bug list; see note |
| L1 | branch dropped / `or` reordered | `bad/provenance-false` (`9:3-12:4` ≠ `11:5-11:24`) | covered |
| L1 | `$value` dropped (key-only range) | `bad/provenance-false` (`11:5-11:16`) | covered |
| L1 | `within "publishConfig"` dropped | no case has a `provenance` node outside `publishConfig` | **AMBIGUOUS / unpinned** — see note |
| L1 | `$value <: not `true`` dropped or flipped | **no case can kill this**: the locator runs only on a matched file, and a file with `"provenance": true` inside `publishConfig` never matches this rule | **GAP (unobservable)** |
| L2 | `$_` → `"provenance": true` | `bad/provenance-false` (`9:3-12:4` ≠ `11:5-11:24`) | covered |

### Proposed cases

1. **`good/other-json`** (killing C1). `packs/npm-provenance/fixtures/provenance/good/other-json/packages/widget/config.json`:
```
{
  "name": "@acme/widget-config",
  "version": "1.0.0"
}
```
Under `r".*\.json"` or an unescaped `package.json`, this file satisfies C2/C3/C4 and starts reporting ⇒ good case fails.

2. **`bad/private-false`** (killing C2 widening). `bad/private-false/packages/widget/package.json`:
```
{
  "name": "@acme/widget",
  "version": "1.0.0",
  "private": false
}
```
`expected-locations.txt`:
```
npm-provenance/provenance packages/widget/package.json
```
(file-only: no `publishConfig`, so neither L1 nor L2 matches — this also pins the fallback.) README contract: "A package counts as public when its `package.json` has a `name` and no `"private": true`."

3. **`good/no-name-package`** (killing C3). `good/no-name-package/packages/widget/package.json`:
```
{
  "version": "1.0.0"
}
```
Dropping C3 makes it report. Contract: the README's "has a `name`" clause. Realism is low (no real published manifest omits `name`), but it is the minimal discriminator; flagged below.

4. **`bad/provenance-string`** (killing C5 widening). `bad/provenance-string/packages/widget/package.json`:
```
{
  "name": "@acme/widget",
  "version": "1.0.0",
  "publishConfig": {
    "access": "public",
    "provenance": "true"
  }
}
```
`expected-locations.txt`:
```
npm-provenance/provenance packages/widget/package.json:6:5-6:25
```
`"provenance": "true"` = 12 + 2 + 6 = 20 chars from col 5 ⇒ end col 25. L1 matches (the value is a string node, so `not `true`` holds) and L2 does not (`"provenance": $_` matches), so the location is unambiguous. C5's textual `not contains "\"provenance\": true"` also reports, which is the rule's stated contract (`"provenance": true`, boolean).

**Unobservable-clause note (L1 `not `true``):** the only state where L1 sees `true` is a manifest that already satisfies C5 — such a manifest never reaches `place` because no `Unlocated` exists. The single textual discriminator would be a manifest written `"provenance":true` (no space): C5's textual `contains` then fails while the node value is boolean `true`. That depends on whether GritQL `contains` on JSON is textual or structural, which I cannot settle by reading the rule file — so I propose no case for it and flag it instead. The same UNSURE applies to any whitespace variant.

### Redundant cases
None. `good/private-package` is the only witness for C2; the other three each kill a distinct C5/C4/locate slip.

---

## Rule 2 — `npm-provenance/public-access`

### Clauses
- **C1** `$name <: r".*package\.json"`
- **C2** `$program <: not contains "\"private": true"`
- **C3** `$program <: contains "\"name": $_"`
- **C4** `$program <: contains "\"name\": $package_name where { $package_name <: r"\"@.*" }`
- **C5a** or-branch A: `not contains "\"publishConfig\": $_"`
- **C5b** or-branch B: `contains "\"publishConfig": $publish where { $publish <: not contains "\"access\": "public" }`

Locate:
- **L1** `"access": $value where { $value <: not `"public"`, $value <: within `"publishConfig": $_` }`
- **L2** `"publishConfig": $publish where { $publish <: not contains `"access": $_` }`

### Existing cases
- `good/private-package` — C2.
- `good/public` — scoped, `access: "public"`: pins C5b negation, C4, `or`→`and`.
- `good/unscoped` — unscoped, no `access`: pins **C4** (dropping the scope conjunct, or `r"\"@.*"` → `r".*"`, or dropping the `\"` escape, makes it report).
- `bad/no-publish-config` — C5a + locate fallback (file-only line).
- `bad/restricted` — `access: "restricted"` ⇒ `10:5-10:27`: pins C5b and L1-over-L2 precedence (L2 would give `9:3-12:4`) and L1's node shape.

### Coverage

| Clause | Plausible bug | Killing case | Verdict |
|---|---|---|---|
| C1 | widen/unescape the file regex | none | **GAP** |
| C2 | `not contains` → `contains` | `good/public`, `good/private-package` | covered |
| C2 | widen to `"private": $_` | none | **GAP** |
| C3 | conjunct dropped | none | **GAP** |
| C4 | scope conjunct dropped / regex unescaped / `r".*"` | `good/unscoped` | covered |
| C4 | whole conjunct dropped (`$package_name` unbound) | `good/unscoped` | covered |
| C5a | branch dropped | `bad/no-publish-config` | covered |
| C5 | `or` → `and` | `good/public` | covered |
| C5b | `not contains` → `contains` | `good/public` | covered |
| C5b | literal widened `"access": "public"` → `"access"` | **no case can kill this**: widening only *reduces* reporting, and the only manifests with a non-`"public"` `access` are already reported (bad) or skipped earlier by C2/C4 | **GAP (unobservable)** |
| L1 | branch dropped / reordered / key-only | `bad/restricted` | covered |
| L1 | `not `"public"`` dropped | **no case can kill this** (same reason as provenance L1: a manifest that reaches the locator never has `access: "public"`) | **GAP (unobservable)** |
| L2 | `$_` → `"access": "public"` | `bad/restricted` | covered |

### Proposed cases

1. **`good/other-json`** (C1). `good/other-json/packages/widget/config.json`:
```
{
  "name": "@acme/widget-config",
  "version": "1.0.0"
}
```
Scoped name and no `publishConfig`, so any widened file regex reports ⇒ good fails.

2. **`bad/private-false`** (C2 widening). `bad/private-false/packages/widget/package.json`:
```
{
  "name": "@acme/widget",
  "version": "1.0.0",
  "private": false
}
```
`expected-locations.txt`:
```
npm-provenance/public-access packages/widget/package.json
```

3. **`good/no-name-package`** (C3). `good/no-name-package/packages/widget/package.json`:
```
{
  "version": "1.0.0"
}
```

### Redundant cases
None. `good/unscoped` is the only witness for C4 and cannot be replaced.

---

## Rule 3 — `npm-provenance/repository-url`

### Clauses
- **C1** `$name <: r".*package\.json"`
- **C2** `$program <: not contains "\"private": true"`
- **C3** `$program <: contains "\"name": $_"`
- **C4a** outer or-branch A: `not contains "\"repository": $_"`
- **C4b** outer or-branch B: `contains "\"repository": $repository where { … }`
- **C5** inner or-branch A: `$repository <: not contains "\"url": $_"`
- **C6** inner or-branch B: `contains "\"url": $url where { $url <: not repositoryUrl() }`

Locate:
- **L1** `"url": $url where { $url <: not repositoryUrl(), $url <: within `"repository": $_` }`
- **L2** `"repository": $repository where { $repository <: not contains `"url": $_` }`

### Existing cases
- `good/private-package` — C2.
- `good/matching-repository` — url equals the `repositoryUrl` param: pins C6 (`not repositoryUrl()`), the outer `or`, and C3.
- `bad/missing-repository` — no `repository`: pins C4a + locate fallback.
- `bad/no-url` — `repository` present with **no** `url`, but a top-level `bugs.url`: pins C5 (C6 replaced by `$program`), L2's range (`7:3-10:4`), and L2's `not contains "url": $_`.
- `bad/other-repository` — url differs from the param ⇒ `6:5-6:51`: pins C6, L1-over-L2 precedence (L2 would not match here at all), L1's node shape.

### Coverage

| Clause | Plausible bug | Killing case | Verdict |
|---|---|---|---|
| C1 | widen/unescape the file regex | none | **GAP** |
| C2 | `not contains` → `contains` | `good/matching-repository`, `good/private-package` | covered |
| C2 | widen to `"private": $_` | none | **GAP** |
| C3 | conjunct dropped | none | **GAP** |
| C4a | branch dropped | `bad/missing-repository` | covered |
| C4 | outer `or` → `and` | `good/matching-repository` | covered |
| C4b | `$repository` replaced by `$program` | `bad/no-url`: `$program` **does** contain `"url": …` (the `bugs.url`), so the scope slip makes `bad/no-url` stop reporting | covered |
| C5 | inner branch dropped / `or` → `and` | `bad/other-repository` (`or`→`and`) and `bad/no-url` (branch dropped) | covered |
| C6 | `not repositoryUrl()` → `repositoryUrl()` | `good/matching-repository` | covered |
| C6 | comparison dropped entirely | `bad/other-repository` | covered |
| C6 | exactness lost (prefix match, or case-insensitive) | none: the only differing url (`…/acme/other.git`) differs in the middle | **GAP** |
| L1 | branch dropped / reordered | `bad/other-repository` (would fall through to L2, which does not match ⇒ file-only) | covered |
| L1 | `$url` dropped (key-only range) | `bad/other-repository` (`6:5-6:17`) | covered |
| L1 | `within "repository": $_` dropped | `bad/no-url` — its `bugs.url` (`https://github.com/acme/widgets/issues`) satisfies `not repositoryUrl()` and would be picked up by a `within`-less L1, changing `7:3-10:4` | covered, **order-dependent** (kills it only if L1's matches are folded before L2's; `place` keeps the first match per path) |
| L1 | `not repositoryUrl()` dropped | `bad/other-repository` (then only `bugs`-style urls would match; here L1 would not match at all ⇒ file-only) | covered |
| L2 | `not contains "url": $_` → `contains` | `bad/no-url` stops reporting | covered |
| L2 | `$repository` replaced by `$program` | none: `$program` in `bad/no-url` contains `"url"` via `bugs` ⇒ the rule would stop reporting, so `bad/no-url` **does** kill this | covered |

### Proposed cases

1. **`good/other-json`** (C1). `good/other-json/packages/widget/config.json`:
```
{
  "name": "@acme/widget-config",
  "version": "1.0.0"
}
```
2. **`bad/private-false`** (C2 widening). `bad/private-false/packages/widget/package.json`:
```
{
  "name": "@acme/widget",
  "version": "1.0.0",
  "private": false
}
```
`expected-locations.txt`:
```
npm-provenance/repository-url packages/widget/package.json
```
3. **`good/no-name-package`** (C3). `good/no-name-package/packages/widget/package.json`:
```
{
  "version": "1.0.0"
}
```
4. **`bad/url-case-mismatch`** (C6 exactness). `bad/url-case-mismatch/packages/widget/package.json`:
```
{
  "name": "@acme/widget",
  "version": "1.0.0",
  "repository": {
    "type": "git",
    "url": "git+https://github.com/acme/widgets.GIT"
  },
  "publishConfig": {
    "access": "public",
    "provenance": true
  }
}
```
`expected-locations.txt`:
```
npm-provenance/repository-url packages/widget/package.json:5:5-5:53
```
`"url": ` (7) + `"git+https://github.com/acme/widgets.GIT"` (41) = 48 chars from col 5 ⇒ end col 53. One case kills both plausible exactness slips: a prefix match on `git+https://github.com/acme/widgets` **and** a case-folded comparison would both accept `…widgets.GIT`, so the bad case would stop reporting. Contract: the rule body states the registry "rejects the provenance bundle when `repository.url` is missing or does not match it exactly (case-sensitive)".

### Redundant cases
None. `good/private-package` is the only C2 witness; `bad/no-url` is the only C5/`$repository`-scope/L2 witness.

---

## Rule 4 — `typecheck-build-mode/typecheck-build-mode`

### Clauses
First bubble:
- **C1** `$name <: r"(.*/)tsconfig\.json"($dir)`
- **C2** `$program <: contains "\"files\": []"`
- **C3** `$program <: contains "\"references\": [$reference, $...]"`

Second bubble:
- **C4** `$name <: r"(.*/)package\.json"($manifest_dir)`
- **C5** `$manifest_dir <: $dir`
- **C6a** or-branch A: `not contains "\"typecheck\": $_"`
- **C6b** or-branch B: `contains "\"typecheck": $script where { $script <: not includes "tsc -b", $script <: not includes "tsc --build" }`

Locate (single pattern, no `or`):
- **L1** `"typecheck": $script where { $script <: not includes "tsc -b", $script <: not includes "tsc --build" }`

### Existing cases
- `good/tsc-build` — `tsc -b`: pins the first `not includes` conjunct.
- `good/tsc-build-long` — `tsc --build`: pins the second conjunct.
- `good/include-config` — `tsconfig.json` with `include`, no `files: []`/`references`, and `tsc --noEmit`: pins C2/C3 (dropping or widening either makes it report) and the regex.
- `bad/tsc-no-emit` — reference-only root + `tsc --noEmit --incremental` ⇒ `3:16-3:57`: pins C6b, both `not includes` conjuncts, and L1's node shape (key-only would give `3:16-3:27`).

### Coverage

| Clause | Plausible bug | Killing case | Verdict |
|---|---|---|---|
| C1 | drop the `\ escape` (`tsconfig.json`) | none: no `tsconfigXjson` file, and `tsconfig.app.json` does not contain the substring `tsconfig.json` | **GAP** (same latent hole as the npm rules' C1, but only reachable with a contrived filename) |
| C1 | **the `(.*/)` prefix requires a `/`** | none: every fixture nests one level | **CRITICAL RULE BUG** — see below |
| C2 | conjunct dropped | `good/include-config` reports | covered |
| C2 | widened to `contains "\"files\""` | none: no fixture has a `files` array that is not `[]` | **GAP** |
| C3 | conjunct dropped | `bad/tsc-no-emit` stops reporting | covered |
| C3 | `[$reference, $...]` widened to `$_` | none: no fixture has `"references": []` | **AMBIGUOUS** — an empty `references` array is a degenerate config; whether it counts as a reference-only root is not stated anywhere. No case proposed |
| C4 | same `(.*/)` anchor as C1 | none | **CRITICAL RULE BUG** (same cause) |
| C5 | `$manifest_dir <: $dir` → `<: not $dir`, or dropped | none | **GAP (unobservable)**: `partition_multifile` (decide.rs:65) puts only same-parent files in one batch, so `$manifest_dir` and `$dir` are *always* equal strings whenever both bubbles see a file. A cross-directory case cannot exist — the two files land in different directory batches |
| C6a | branch dropped | none: no fixture omits `typecheck` | **GAP** — and the README explicitly claims it ("or no script at all") |
| C6 | `or` → `and` | none: every fixture has a `typecheck` script | same **GAP** as C6a |
| C6b | conjunct `not includes "tsc -b"` dropped | `good/tsc-build` reports | covered |
| C6b | conjunct `not includes "tsc --build"` dropped | `good/tsc-build-long` reports | covered |
| C6b | `includes` → exact match on the script | `bad/tsc-no-emit` (`tsc --noEmit --incremental`) stays reported, `good/tsc-build` stays clean | **GAP (unobservable)** — `includes` and equality agree on all three fixture scripts; only a compound script (`"typecheck": "tsc -b --verbose"`, which the rule accepts) distinguishes them. Not proposed: the rule text does not commit either way |
| L1 | node shape (key-only) | `bad/tsc-no-emit` (`3:16-3:27`) | covered |
| L1 | either `not includes` conjunct dropped | **no case can kill this** — as with provenance L1, a script containing `tsc -b`/`tsc --build` never reaches the locator | **GAP (unobservable)** |
| L1 | missing branch for the no-`typecheck` case | n/a | **AMBIGUOUS** — see below |

### CRITICAL finding: the rule never fires for a repo-root tsconfig

Both file predicates capture `$name <: r"(.*/)tsconfig\.json"` and `$name <: r"(.*/)package\.json"`. Fixture paths are `packages/app/…`, so `.*/` matches. A root-level manifest has path `tsconfig.json`, which contains **no `/`**, so C1 (and C4) cannot match and the archetypal monorepo — root `tsconfig.json` + root `package.json` — is never reported. This is a rule defect, not a fixture gap: the minimal case that would expose it **cannot pass today**. Recommended rule change: `r"(.*/?)tsconfig\.json"` and `r"(.*/?)package\.json"` (with `$dir` then `""` for a root file, and `$manifest_dir <: $dir` still holding). No fixture is proposed for it here.

### Proposed cases

1. **`bad/no-typecheck-script`** (killing C6a / `or`→`and`). Three files under `bad/no-typecheck-script/packages/app/`:

`tsconfig.json`
```
{
  "files": [],
  "references": [{ "path": "./tsconfig.app.json" }]
}
```
`tsconfig.app.json`
```
{
  "compilerOptions": { "composite": true },
  "include": ["src"]
}
```
`package.json`
```
{
  "name": "@acme/app",
  "scripts": { "lint": "eslint ." }
}
```
`expected-locations.txt`:
```
typecheck-build-mode/typecheck-build-mode packages/app/package.json
```
File-only by design: with no `typecheck` key, L1 has nothing to match and `place` falls back to the matched path. This case also pins that documented behaviour ("or no script at all", README) and that the harness accepts a file-only line for a `locate`-bearing rule.

2. **`bad/files-non-empty`** (killing C2 widening). `bad/files-non-empty/packages/app/tsconfig.json`:
```
{
  "files": ["scripts/build.ts"],
  "references": [{ "path": "./tsconfig.app.json" }]
}
```
`bad/files-non-empty/packages/app/tsconfig.app.json`
```
{
  "compilerOptions": { "composite": true },
  "include": ["src"]
}
```
`bad/files-non-empty/packages/app/package.json`
```
{
  "name": "@acme/app",
  "scripts": { "typecheck": "tsc --noEmit" }
}
```
`expected-locations.txt`:
```
typecheck-build-mode/typecheck-build-mode packages/app/package.json:3:16-3:44
```
`"typecheck"` starts at col 16 (`  "scripts": { ` is 15 chars); `"typecheck": "tsc --noEmit"` is 12 + 2 + 14 = 28 chars ⇒ end col 44. Under a widened C2 (`contains "\"files\""`) this root matches and reports; under the exact C2 it is clean-by-rule and correctly reported as bad. Realism is moderate (a root that owns a bootstrap file *and* references projects), flagged accordingly.

### Redundant cases
None. `good/tsc-build` and `good/tsc-build-long` each kill a distinct conjunct; `good/include-config` is the only C2/C3 witness; `bad/tsc-no-emit` is the only location witness.

---

## Flagged ambiguities (no case proposed)

1. **`provenance` L1 `$value <: not `true``** and **`public-access` L1 `$value <: not `"public"``**, and both `typecheck-build-mode` L1 `not includes` conjuncts — the locator only runs on already-flagged manifests, so these conjuncts can never change an outcome. Either drop them from `locate` or add a whitespace-form case (UNSURE: whether `contains` on JSON is textual or structural decides it).
2. **`typecheck-build-mode` locate has no branch for a missing `typecheck` script**, yet the rule and README both claim to report that case. Whether such findings *should* be located (e.g. on `"scripts": $_`) is undecided; today they are necessarily file-only.
3. **`typecheck-build-mode` C3's `[$reference, $...]`** — whether `"references": []` counts as a reference-only root is unspecified.
4. **`typecheck-build-mode` C6b `includes` vs exact match** — the rule accepts `"tsc -b --verbose"`; whether that is intended is unstated, and no fixture pins it.
5. **C2/C3 realism** — the two proposed "minimal discriminator" shapes per npm rule (`"private": false`, a manifest with no `name`) are contract-backed by the README but do not occur in real published packages. They are the smallest fixtures that kill the widening slips; if realism is preferred, `bad/private-false` is the one worth keeping and the `no-name` cases could be dropped at the cost of two gaps.

4 rules, 13 gaps, 13 proposed cases