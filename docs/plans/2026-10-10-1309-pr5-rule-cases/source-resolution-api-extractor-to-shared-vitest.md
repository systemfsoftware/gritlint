# source-resolution: test-gap analysis (5 rules)

Read-only analysis per `local://pr5-gap-method.md`. Every claim below is grounded in
`packs/source-resolution/rules/<rule>.md`, the fixture files, and
`crates/gritlint_core/src/decide.rs` (`evaluate` compiles **one** rule per case and compares the
sorted set of `expected-locations.txt` lines, so a case pins exactly one rule's findings and
nothing else).

**Range convention** (verified against committed expectations, not assumed):
`api-extractor-tsconfig` bad case expects `tsconfig.api.json:4:5-4:52` for the line
`    "customConditions": ["@systemfsoftware/source"]` (length 51, starts at column 5) — so the
start column is 1-based inclusive and **the end column is exclusive** (`len+1`). Same for
`publish-config-exports` `12:7-12:38` (line length 37) and `shared-vitest-config` `2:14-2:43`
(`conditions: ['@other/source']` is 29 columns starting at 14).

**Common case scaffolding.** Every proposed case directory needs its own `gritlint.json` with the
five required parameters (all five are `requiredParams` in `pack.json` and every existing case
carries them):

```json
{
  "packs": {
    "source-resolution": {
      "condition": "r\"['\\\"]@systemfsoftware/source['\\\"]\"",
      "presets": "r\"\\\"@systemfsoftware/tsconfig/.*\\\"\"",
      "presetFiles": "r\".*/tsconfig/(?:(?:bundler|tsc)/.*|node)\\.json\"",
      "sharedVitestConfig": "r\"['\\\"]@systemfsoftware/vitest-config['\\\"]\"",
      "sharedVitestConfigFile": "r\".*/vitest-config/lib/base\\.js\""
    }
  }
}
```

Every proposed `packages/app/tsdown.config.ts` is the existing two-line form:

```ts
export default defineConfig({
  exports: { devExports: '@systemfsoftware/source', customExports: injectTypes },
})
```

---

## source-resolution/api-extractor-tsconfig

### Clauses

| id | clause text |
|----|-------------|
| C1 | `bubble($dir) file($name, $body) where { $name <: r"(.*/)api-extractor\.json"($dir), ... }` |
| C2 | `$program <: contains `"tsconfigFilePath": $_`` (on the `api-extractor.json`) |
| C3 | `bubble($api_dir) file($name, $body) where { $name <: r"(.*/)tsconfig\.api\.json"($api_dir), ... }` |
| C4 | `$api_dir <: $dir` |
| C5 | `$program <: not contains `"customConditions": []`` (on the `tsconfig.api.json`) |
| L1 | `locate: `"customConditions": $value` where { $value <: not `[]` }` |

### Cases

- `good/clears-condition` — api-extractor points at a sibling `tsconfig.api.json` whose
  `customConditions` is `[]`; nothing reported.
- `bad/keeps-condition` — same pair, `["@systemfsoftware/source"]`; one finding at
  `tsconfig.api.json:4:5-4:52`.

### Coverage

| clause | killing case / GAP |
|--------|--------------------|
| C1 | `bad/keeps-condition` kills dropping the `(.*/)` capture or any structural break of the
  regex (the rule would go clean). **GAP-A**: losing the `\.` escape (`api-extractor.json`) is
  unkillable today — no fixture contains a sibling name that only the unescaped regex matches.
  Proposal D below. |
| C2 | **GAP-B**: no case exists in which `api-extractor.json` lacks `"tsconfigFilePath"`, so
  dropping the conjunct (or mistyping the key) changes nothing in any case. Proposal A below. |
| C3 | `bad/keeps-condition` kills structural breaks. **GAP-C**: escaped-dot loss, as C1.
  Proposal D below. |
| C4 | **GAP-D**: both cases are single-directory, so `$api_dir <: $dir` can be dropped or
  loosened without any case changing. Proposal B below. |
| C5 | `good/clears-condition` kills dropping the conjunct or the `not`
  (`contains "customConditions": []` would report the good case); `bad/keeps-condition` kills
  widening it to `"customConditions"` key-presence. Covered. |
| L1 | the node itself is pinned by `bad/keeps-condition`'s `4:5-4:52` (the `"key": value` pair),
  so widening `$value` or matching the wrong node fails. **GAP-E**: the `$value <: not []`
  conjunct is not killable by any case in the shape C5 permits — C5 requires the *contiguous* text
  `"customConditions": []` to be absent, so the only way to make a `[]` node exist without C5
  being satisfied is a line break between `:` and `[]`. Proposal F (artificial) below. |

### Proposed cases

**A. `good/no-tsconfig-file-path`** — kills GAP-B (C2 dropped or re-keyed: `api-extractor.json`
present, `tsconfig.api.json` keeps the condition, but no `compiler.tsconfigFilePath` → today silent,
after the mutation a finding).
This case also pins a real false negative: with no `tsconfigFilePath` the rule is silent, which is
arguably also a defect (`nextAction` says the extractor must *point at* that file). Flagging it so
the fixture is committed knowingly.

`packages/app/api-extractor.json`
```json
{
  "projectFolder": "."
}
```
`packages/app/tsconfig.api.json`
```json
{
  "extends": "./tsconfig.build.json",
  "compilerOptions": {
    "customConditions": ["@systemfsoftware/source"]
  }
}
```

**B. `good/api-tsconfig-elsewhere`** — kills GAP-D (C4 dropped: the `packages/lib` api tsconfig
joins the `packages/app` extractor and is reported).

`packages/app/api-extractor.json`
```json
{
  "projectFolder": ".",
  "compiler": {
    "tsconfigFilePath": "<projectFolder>/tsconfig.api.json"
  }
}
```
`packages/app/tsconfig.api.json`
```json
{
  "extends": "./tsconfig.build.json",
  "compilerOptions": {
    "customConditions": []
  }
}
```
`packages/lib/tsconfig.api.json`
```json
{
  "extends": "./tsconfig.build.json",
  "compilerOptions": {
    "customConditions": ["@systemfsoftware/source"]
  }
}
```

**C/D. `good/decoy-file-names`** — kills GAP-A and GAP-C together (each decoy is isolated in its own
directory so either escape loss alone is caught): in `packages/a`, `api-extractorxjson` matches only
the unescaped C1; in `packages/b`, `tsconfig.apixjson` matches only the unescaped C3.

`packages/a/api-extractorxjson`
```json
{
  "projectFolder": ".",
  "compiler": {
    "tsconfigFilePath": "<projectFolder>/tsconfig.api.json"
  }
}
```
`packages/a/tsconfig.api.json`
```json
{
  "extends": "./tsconfig.build.json",
  "compilerOptions": {
    "customConditions": ["@systemfsoftware/source"]
  }
}
```
`packages/b/api-extractor.json`
```json
{
  "projectFolder": ".",
  "compiler": {
    "tsconfigFilePath": "<projectFolder>/tsconfig.api.json"
  }
}
```
`packages/b/tsconfig.apixjson`
```json
{
  "extends": "./tsconfig.build.json",
  "compilerOptions": {
    "customConditions": ["@systemfsoftware/source"]
  }
}
```

**F. `bad/cleared-condition-decoy`** — kills GAP-E (L1's `not []` dropped: the locator would report
`4:9-5:13`, the decoy, instead of `9:5-9:52`). The case is deliberately unformatted; mark it
artificial, it is only worth committing if the maintainers want L1 pinned.

`packages/app/api-extractor.json`
```json
{
  "projectFolder": ".",
  "compiler": {
    "tsconfigFilePath": "<projectFolder>/tsconfig.api.json"
  }
}
```
`packages/app/tsconfig.api.json`
```json
{
  "references": [
    {
      "customConditions":
        []
    }
  ],
  "compilerOptions": {
    "customConditions": ["@systemfsoftware/source"]
  }
}
```
`expected-locations.txt`
```
source-resolution/api-extractor-tsconfig packages/app/tsconfig.api.json:9:5-9:52
```

### Redundant cases

None. Both cases kill something (`bad/keeps-condition`: C1/C3/C5/L1; `good/clears-condition`: C5).

---

## source-resolution/export-map-order

### Clauses

| id | clause text |
|----|-------------|
| C1 | `$name <: r"(.*/)tsdown\.config\.ts"($dir)` (bubble 1 — the consumer-safe gate) |
| C2 | `$name <: r"(.*/)package\.json"($package_dir)` |
| C3 | `$package_dir <: $dir` |
| C4 | `$program <: contains `"exports": $exports`` |
| B1 | `contains `$subpath: { "types": $_, $cond: $_, "default": $_ }` where { $cond <: condition() }` |
| B2 | `contains `$subpath: { $cond: $_, "default": $_, "types": $_ }` where { $cond <: condition() }` |
| B3 | `contains `$subpath: { $cond: $_, "types": $target, "default": $_ }` where { $cond <: condition(), $target <: not includes "./dist/" }` |
| B4 | `contains `$subpath: { $first: $_, $second: $_, "default": $_ }` where { $first <: not condition() }` |
| B5 | `contains `$subpath: { "types": $_, $cond: $_ }` where { $cond <: condition() }` |
| B6 | `contains `$subpath: { $cond: $_, "types": $_ }` where { $cond <: condition() }` |
| B7 | `contains `$subpath: { $cond: $_, "default": $_ }` where { $cond <: condition() }` |
| L1 | `locate: ... where { $pair <: within `"exports": $_`, $pair <: or { B1..B7 } }` |

The `or` in the locator repeats B1..B7 verbatim, so a branch dropped only in `locate` turns its bad
case's expected range into a file-alone finding — the B2/B3/B5/B6/B7 proposals below pin both copies.

### Cases

- `good/condition-first` — `"."` = condition, `types`, `default`; `./package.json` a bare string.
  Kills C4 (dropped → reports), B4's `$first <: not condition()` negation, B3's `not includes
  "./dist/"` negation, and every B-branch widening (unconstrained `$cond`).
- `bad/types-before-condition` — `"./tester"` = `types`, condition, `default`; one finding at
  `package.json:4:5-8:6`.

### Coverage

| clause | killing case / GAP |
|--------|--------------------|
| C1 | structural breaks killed by `bad/types-before-condition`. **GAP-F**: no case holds a
  misordered map in a directory whose bundler-config lookup finds nothing (or finds a *different*
  `.ts` file), so removing/loosening the gate is undetected. Proposal H. |
| C2 | structural breaks killed by `bad/types-before-condition`. **GAP-G**: escaped-dot loss
  (`package.json`) unkillable. Proposal I. |
| C3 | **GAP-H** (shared with C1): every case is single-directory. Proposal H. |
| C4 | `good/condition-first` (dropped → reports) and `bad/types-before-condition`
  (re-keyed to `main`/`imports` → clean). Covered. |
| B1 | **Unkillable, and redundant.** Every B1 match has `"types"` as its first key, so `$first` is
  never `condition()` and B4 matches it too: B1 ⊆ B4. Dropping B1 leaves `bad/types-before-condition`
  reporting at the same range. No case can distinguish them unless object patterns accept extra keys
  (unstated engine semantics — flagging, not assuming). Recommend deleting B1 or documenting it as
  documentation-only. |
| B2 | **GAP-I**: no fixture has condition → `default` → `types`. Proposal A. |
| B3 | **GAP-J**: no fixture has a correct first key with `types` outside `./dist/`. Proposal B. |
| B4 | **GAP-K**: generality untested — every bad fixture's first key is `"types"`, which B1 also
  matches, so dropping B4 still reports. Proposal C. |
| B5 | **GAP-L**: no fixture has `types` + condition with no `default`. Proposal D. |
| B6 | **GAP-M**: no fixture has condition + `types` with no `default`. Proposal E. |
| B7 | **GAP-N**: no fixture has condition + `default` with no `types`. Proposal F. |
| L1 | `$pair` and the branch set are pinned by `bad/types-before-condition` (`4:5-8:6`).
  **GAP-O**: `within `"exports": $_` is never distinguished — the good case's `./package.json`
  pair matches no branch, and the bad case has no out-of-`exports` pair. Proposal G kills
  `within "publishConfig"` / `within "imports"`; it does **not** kill deleting `within` altogether
  (the first in-file match would still be the `exports` pair). Residual noted, not closable without
  an implausible field order. |

### Proposed cases

All seven bad cases need `packages/app/tsdown.config.ts` (content above).

**A. `bad/default-before-types`** — kills GAP-I (B2; B1 needs `types` first, B4 excludes a
condition first key, B3 needs `types` third).
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "default": "./dist/index.mjs",
      "types": "./dist/index.d.ts"
    }
  }
}
```
```
source-resolution/export-map-order packages/app/package.json:4:5-8:6
```

**B. `bad/types-not-dist`** — kills GAP-J (B3: condition first, `types` pointing at `./src/`).
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./src/index.d.ts",
      "default": "./dist/index.mjs"
    }
  }
}
```
```
source-resolution/export-map-order packages/app/package.json:4:5-8:6
```

**C. `bad/foreign-condition-first`** — kills GAP-K (B4: first key neither `"types"` nor the
condition, so B1 cannot match).
```json
{
  "name": "@acme/app",
  "exports": {
    "./tester": {
      "node": "./dist/tester.mjs",
      "@systemfsoftware/source": "./src/tester.ts",
      "default": "./dist/tester.mjs"
    }
  }
}
```
```
source-resolution/export-map-order packages/app/package.json:4:5-8:6
```

**D. `bad/types-without-default`** — kills GAP-L (B5 only; B4 needs three keys).
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "@systemfsoftware/source": "./src/index.ts"
    }
  }
}
```
```
source-resolution/export-map-order packages/app/package.json:4:5-7:6
```

**E. `bad/condition-and-types`** — kills GAP-M (B6 only).
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts"
    }
  }
}
```
```
source-resolution/export-map-order packages/app/package.json:4:5-7:6
```

**F. `bad/condition-and-default`** — kills GAP-N (B7 only).
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "default": "./dist/index.mjs"
    }
  }
}
```
```
source-resolution/export-map-order packages/app/package.json:4:5-7:6
```

**G. `bad/locate-scope`** — kills GAP-O: the same misordered pair exists in `exports` and in
`publishConfig.exports`, so a locator anchored on the wrong object reports `4:5-4:31` instead of
`13:7-13:26`.
```json
{
  "name": "@acme/app",
  "exports": {
    "./tester": "./dist/tester.mjs",
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    }
  },
  "publishConfig": {
    "exports": {
      "./tester": "./dist/tester.mjs"
    }
  }
}
```
```
source-resolution/export-map-order packages/app/package.json:13:7-13:26
```

**H. `good/misordered-map-untouched`** — kills GAP-F and GAP-H: `packages/lib` has a misordered map
and a *different* bundler config (so a loosened C1 matches), `packages/tool` has a well-formed
`tsdown.config.ts` and a clean map (so a dropped C3 joins the two).
`packages/lib/package.json`
```json
{
  "name": "@acme/lib",
  "exports": {
    "./tester": {
      "types": "./dist/tester.d.ts",
      "@systemfsoftware/source": "./src/tester.ts",
      "default": "./dist/tester.mjs"
    }
  }
}
```
`packages/lib/vite.config.ts`
```ts
export default defineConfig({ build: { lib: { entry: 'src/index.ts' } } })
```
`packages/tool/tsdown.config.ts` (the standard two-liner above)
`packages/tool/package.json`
```json
{
  "name": "@acme/tool",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    }
  }
}
```

**I. `good/escaped-file-names`** — kills GAP-G, one escape per directory:
`packages/a/package_json` matches only the unescaped C2 and `packages/a/tsdown_config_ts` only the
unescaped C1; `packages/b/tsdown.config.ts` is real and `packages/b/package_json` holds a misordered
map.
`packages/a/tsdown_config_ts`
```ts
export default defineConfig({
  exports: { devExports: '@systemfsoftware/source', customExports: injectTypes },
})
```
`packages/a/package_json`
```json
{
  "name": "@acme/a",
  "exports": {
    "./tester": {
      "types": "./dist/tester.d.ts",
      "@systemfsoftware/source": "./src/tester.ts",
      "default": "./dist/tester.mjs"
    }
  }
}
```
`packages/b/tsdown.config.ts` (the standard two-liner above)
`packages/b/package_json`
```json
{
  "name": "@acme/b",
  "exports": {
    "./tester": {
      "types": "./dist/tester.d.ts",
      "@systemfsoftware/source": "./src/tester.ts",
      "default": "./dist/tester.mjs"
    }
  }
}
```

### Redundant cases

Neither. Note the asymmetry: `bad/types-before-condition` cannot fail if B1 *or* B4 is dropped, so
the committed bad case pins only "some branch reports here", not which. The `"./package.json"`
bare-string subpath in `good/condition-first` is inert for this rule (no branch matches bare
strings — that is `publish-config-exports`' job); harmless, but it buys no mutation coverage.

---

## source-resolution/internals-export-map

### Clauses

| id | clause text |
|----|-------------|
| C1 | `bubble($dir, $tsdown) file($name, $body) where { if ($name <: r"(.*/)tsdown\.config\.ts"($dir)) { $tsdown = $filename } else { true } }` |
| C2 | `$name <: r"(.*/)package\.json"($dir)` (bubble 2, joined to bubble 1 by the shared `$dir` capture) |
| C3 | `$program <: contains `"exports": $exports` where { $exports <: contains `$subpath: { $cond: $_, "types": $_, "default": $_ }` where { $cond <: condition() } }` |
| C4 | `$tsdown <: undefined` |
| L1 | `locate: `$subpath: { $cond: $_, "types": $_, "default": $_ }` where { $cond <: condition() }` |

### Cases

- `good/consumer-safe` — same three-key condition-first map as the bad case, but a sibling
  `tsdown.config.ts` exists; silent. Kills C1 (both the `if` and the `else` branch) and C4.
- `bad/internals-carries-condition` — no `tsdown.config.ts`; one finding at
  `package.json:4:5-8:6`. Kills C1's `$tsdown = $filename` assignment, C2, C3 and L1's key set.
- `good/internals-dist-only` — internals package whose only subpath is `{ "types", "default" }`.

### Coverage

| clause | killing case / GAP |
|--------|--------------------|
| C1 | `good/consumer-safe` kills dropping the `else` or inverting `if`/`else`; `bad/internals-carries-condition`
  kills dropping the assignment or `$filename`. **GAP-P**: escaped-dot loss in
  `tsdown\.config\.ts` is unkillable. Proposal B. |
| C2 | `bad/internals-carries-condition` kills structural breaks. **GAP-Q**: escaped-dot loss in
  `package\.json` is unkillable. Proposal C. |
| C3 | **GAP-R**: `$cond <: condition()` has no killing case — `good/internals-dist-only` has only
  two keys, so widening `$cond` to any key still finds nothing, and `bad/internals-carries-condition`
  matches with or without the predicate. Also kills the same conjunct in L1 (the good case would
  start reporting). Proposal A. |
| C4 | `good/consumer-safe` (dropped → the internals package is judged as consumer-safe → silent,
  so that case still passes — but `bad/internals-carries-condition` would go clean, which is a
  failure). Covered. |
| L1 | key set and `$pair` pinned by `4:5-8:6` in the bad case; order, `not []`-style and predicate
  slips pinned by Proposal A. Covered once A is committed. |

### Proposed cases

**A. `good/internals-other-condition`** — kills GAP-R: a three-key internals subpath whose first key
is a *different* condition. Correct rule silent; widening or negating `$cond <: condition()` reports
it (and locates it).
`packages/app/package.json`
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@other/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    }
  }
}
```

**B. `bad/decoy-bundler-name`** — kills GAP-P: the only bundler-looking sibling is
`tsdown_config_ts`, which the escaped regex rejects, so the package is internals and is reported;
with the escape lost, `$tsdown` is defined and the rule goes clean.
`packages/app/tsdown_config_ts`
```ts
export default defineConfig({
  exports: { devExports: '@systemfsoftware/source', customExports: injectTypes },
})
```
`packages/app/package.json`
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    }
  }
}
```
```
source-resolution/internals-export-map packages/app/package.json:4:5-8:6
```

**C. `good/decoy-manifest-name`** — kills GAP-Q: a real `tsdown.config.ts` beside
`package_json` (not `package.json`); silent today, reported if the escape is lost.
`packages/app/tsdown.config.ts` (the standard two-liner above)
`packages/app/package_json`
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    }
  }
}
```

### Redundant cases

- `good/internals-dist-only` is redundant **today**: it is the only good case for this rule, and no
  plausible one-step slip makes it report. Its subpath has two keys, so every three-key pattern in
  C3/L1 misses it whether or not `$cond <: condition()` is present; `$tsdown` is `undefined` in both
  directions of the `if`; dropping C4 leaves it silent. It documents intent ("internals stay
  `types`/`default` to `dist/`") but pins nothing. Proposal A supersedes its mutation value — keep it
  for documentation, but do not count it as coverage.
- `good/consumer-safe` is **not** redundant (kills C1's `else`, the inverting slip, and `$tsdown = $name`).

---

## source-resolution/publish-config-exports

### Clauses

| id | clause text |
|----|-------------|
| C1 | `bubble($dir) file($name, $body) where { $name <: r"(.*/)tsdown\.config\.ts"($dir) }` |
| C2 | `$name <: r"(.*/)package\.json"($package_dir)` |
| C3 | `$package_dir <: $dir` |
| C4 | `or { $program <: not contains `"publishConfig": $_`, ... }` (publish map required) |
| C5 | `$publish <: not contains `"exports": $_`` (an exports-less publishConfig is a defect) |
| B1 | `$exports <: or { contains `$subpath: $entry` where { $subpath <: or { `"."`, includes "./" }, $subpath <: not includes "./package.json", $entry <: `"$bare"` }, ... }` |
| B2 | `contains `$subpath: $entry` where { $subpath <: or { `"."`, includes "./" }, $entry <: contains `$cond: $_` where { $cond <: condition() } }` |
| L1 | `locate: or { `$subpath: $entry` where { B1 clauses, $entry <: within `"publishConfig": $_` }, `$subpath: $entry` where { B2 clauses, $entry <: within `"publishConfig": $_` }, `"publishConfig": $publish` where { $publish <: not contains `"exports": $_` } }` |

### Cases

- `bad/bare-string` — `publishConfig.exports["./tester"]` is the string `"./dist/tester.mjs"`;
  finding at `12:7-12:38`.
- `bad/leaks-condition` — `publishConfig.exports["."]` carries the condition; finding at
  `12:7-16:8`.
- `good/dist-only` — `publishConfig.exports` = `"."` `{types, default}` + the bare
  `"./package.json"`; also holds `"provenance": true`.

### Coverage

| clause | killing case / GAP |
|--------|--------------------|
| C1 | structural breaks killed by both bad cases (no `$dir` → rule never matches). **GAP-S**:
  escaped-dot loss and gate widening — Proposal F. |
| C2 | structural breaks killed by both bad cases. **GAP-T**: escaped-dot loss — Proposal F. |
| C3 | **GAP-U**: every case is single-directory. Proposal F. |
| C4 | negated-branch slips (`not` dropped, branch deleted) only *lose* reports, so only a bad case
  can catch them, and both existing bad cases have a `publishConfig`. **GAP-V**: no case for a
  consumer-safe package with **no** `publishConfig`. Proposal A. |
| C5 | **GAP-W**: same reasoning — no case has a `publishConfig` without an `exports` key. Proposal B. |
| B1 | `bad/bare-string` kills `includes "./"` dropped (its subpath is `"./tester"`, not `"."`),
  `or`→`and`, and `$entry <: `"$bare"`` flips. `good/dist-only` kills `$subpath <: not includes
  "./package.json"` dropped. **GAP-X**: the `"."` half of the subpath `or` is untested — the only
  bare-string case uses `"./tester"`. Proposal C. |
| B2 | `bad/leaks-condition` kills the `$cond <: condition()` predicate dropped (any key of
  `good/dist-only`'s publish entry would match). **GAP-Y**: the `includes "./"` half of B2's subpath
  `or` is untested — the only leak case uses `"."`. Proposal D. |
| L1 | `within `"publishConfig": $_`` on both entry branches is never distinguished. **GAP-Z**:
  Proposal E. |

### Proposed cases

**A. `bad/no-publish-config`** — kills GAP-V (C4's `not contains "publishConfig"` branch dropped →
today silent, after the mutation reported). It also pins L1's file-alone fallback: the locator has
no `"publishConfig"` node to anchor on, so the finding is reported at the first matched path, which
in path order is `package.json` (before `tsdown.config.ts`).
`packages/app/package.json`
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    },
    "./package.json": "./package.json"
  }
}
```
`packages/app/tsdown.config.ts` (the standard two-liner above)
```
source-resolution/publish-config-exports packages/app/package.json
```

**B. `bad/publish-config-without-exports`** — kills GAP-W (C5 dropped or its `not` dropped: the rule
would go clean). Also pins L1's third branch range.
`packages/app/package.json`
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    },
    "./package.json": "./package.json"
  },
  "publishConfig": {
    "access": "public"
  }
}
```
`packages/app/tsdown.config.ts` (the standard two-liner above)
```
source-resolution/publish-config-exports packages/app/package.json:11:3-13:4
```

**C. `bad/bare-string-dot`** — kills GAP-X: a bare string under the `"."` key, which only the `"."`
half of the subpath `or` can reach.
`packages/app/package.json`
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    },
    "./package.json": "./package.json"
  },
  "publishConfig": {
    "exports": {
      ".": "./dist/index.mjs"
    }
  }
}
```
`packages/app/tsdown.config.ts` (the standard two-liner above)
```
source-resolution/publish-config-exports packages/app/package.json:13:7-13:26
```

**D. `bad/leaks-condition-subpath`** — kills GAP-Y: the condition under `"./tester"`, which only the
`includes "./"` half can reach.
`packages/app/package.json`
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    },
    "./package.json": "./package.json"
  },
  "publishConfig": {
    "exports": {
      "./tester": {
        "@systemfsoftware/source": "./src/tester.ts",
        "types": "./dist/tester.d.ts",
        "default": "./dist/tester.mjs"
      }
    }
  }
}
```
`packages/app/tsdown.config.ts` (the standard two-liner above)
```
source-resolution/publish-config-exports packages/app/package.json:13:7-17:8
```

**E. `bad/locate-scope`** — kills GAP-Z: the same bare string sits in `exports` and in
`publishConfig.exports`, so a locator anchored on (or widened past) the wrong object reports
`4:5-4:31` instead of `13:7-13:26`.
`packages/app/package.json`
```json
{
  "name": "@acme/app",
  "exports": {
    "./tester": "./dist/tester.mjs",
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    }
  },
  "publishConfig": {
    "exports": {
      "./tester": "./dist/tester.mjs"
    }
  }
}
```
`packages/app/tsdown.config.ts` (the standard two-liner above)
```
source-resolution/publish-config-exports packages/app/package.json:13:7-13:26
```

**F. `good/publish-config-untouched-elsewhere`** — kills GAP-S, GAP-T and GAP-U at once.
`packages/lib` has a hand-maintained `publishConfig` with a bare string and *no* tsdown config, so a
loosened or unescaped C1 reports it; a decoy `tsdown_config_ts` there covers C1's escape.
`packages/tool` has a real `tsdown.config.ts` and a decoy `package_json`, so a dropped C3 or an
unescaped C2 reports it.
`packages/lib/package.json`
```json
{
  "name": "@acme/lib",
  "exports": {
    ".": {
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    }
  },
  "publishConfig": {
    "exports": {
      "./tester": "./dist/tester.mjs"
    }
  }
}
```
`packages/lib/tsdown_config_ts`
```ts
export default defineConfig({
  exports: { devExports: '@systemfsoftware/source', customExports: injectTypes },
})
```
`packages/tool/tsdown.config.ts` (the standard two-liner above)
`packages/tool/package_json`
```json
{
  "name": "@acme/tool",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    }
  },
  "publishConfig": {
    "exports": {
      "./tester": "./dist/tester.mjs"
    }
  }
}
```

**G. `good/publish-config-other-condition`** — kills widening of `$cond <: condition()` in B2 (the
loosest form, `$_`, is already caught by `good/dist-only`; this pins a foreign condition string).
Lower value than A–F; commit only if the pack wants that boundary asserted.
`packages/app/package.json`
```json
{
  "name": "@acme/app",
  "exports": {
    ".": {
      "@systemfsoftware/source": "./src/index.ts",
      "types": "./dist/index.d.ts",
      "default": "./dist/index.mjs"
    }
  },
  "publishConfig": {
    "exports": {
      ".": {
        "@other/source": "./src/index.ts",
        "types": "./dist/index.d.ts",
        "default": "./dist/index.mjs"
      }
    }
  }
}
```
`packages/app/tsdown.config.ts` (the standard two-liner above)

### Redundant cases

None. `good/dist-only` is the load-bearing case: it alone kills C4's `not` flip, C5's `not` flip, B1's
`./package.json` exclusion and B2's predicate.

---

## source-resolution/shared-vitest-config

### Clauses

| id | clause text |
|----|-------------|
| C1 | `$name <: r".*/vitest-config/lib/base\.js"` |
| C2 | `$name <: sharedVitestConfigFile()` |
| C3a | `$program <: or { not contains condition(), ... }` (first branch) |
| C3b | `not contains `resolve: { conditions: $_ }`` |
| C3c | `not contains `ssr: { resolve: { conditions: $_ } }`` |
| L1a | `locate: ... where { $list <: not contains condition(), ... }` |
| L1b | `$program <: not contains condition()` |

### Cases

- `good/both-keys` — the condition is a named const, both pipelines wired. Kills C3b/C3c `not` drops.
- `bad/conditions-without-condition` — both pipelines carry `@other/source`; the program has no
  `condition()` node at all, so C3a fires; the locator anchors on the first `conditions:` node →
  `base.js:2:14-2:43`. Kills C3a.
- `bad/missing-ssr` — only `resolve.conditions`, but the const declares the condition, so C3c fires
  while L1b fails and the finding falls back to the file alone. Kills C3c (removing the branch) and
  L1b.

### Coverage

| clause | killing case / GAP |
|--------|--------------------|
| C1 | **GAP-AA**: the conjunct is never distinguished from C2 — with the default parameters both
  are `.*/vitest-config/lib/base\.js`, so removing C1 changes nothing. Only a case that *re-points*
  `sharedVitestConfigFile` can catch it. Proposal A. |
| C2 | **GAP-AB**: every case uses the default `sharedVitestConfigFile`, so the parameter itself is
  never exercised as a distinct value. Proposal A. |
| C1 escape (`base\.js`) | **GAP-AC**: unkillable while C2's parameter still carries its own escaped
  default; Proposal B re-points the parameter at `baseXjs` so the rule's literal becomes the only
  thing standing between the case and a report. |
| C3a | `bad/conditions-without-condition`. Covered. |
| C3b | `good/both-keys` (`not` dropped → the good case reports). Covered. |
| C3c | `good/both-keys` (`not` dropped) and `bad/missing-ssr` (branch deleted → clean). Covered. |
| `or`→`and` | both bad cases (a file satisfying one branch and falsifying another goes clean). Covered. |
| L1a | **GAP-AD — unkillable, not a fixture problem.** `$list <: not contains condition()` can never
  decide anything: L1b already requires `$program <: not contains condition()`, and a list is a
  subset of the program, so whenever L1a is false L1b is false too. The conjunct is effectively dead.
  Report it as dead code (or as an unenforceable assumption the engine may not guarantee) rather
  than proposing an artificial case — no input distinguishes the two forms. |
| L1b | `bad/missing-ssr` (dropped → the locator would match line 3 and the expected range becomes
  file-alone). Covered. |
| locate node | `bad/conditions-without-condition`'s `2:14-2:43` pins `conditions: $list`; widening the
  key or anchoring at the wrong node fails. Covered. |

### Proposed cases

**A. `good/parameter-selects-shared-config`** — kills GAP-AA and GAP-AB: the parameter is
re-pointed at `main.js`; `base.js` is broken and `main.js` is broken. Today nothing matches both C1
and C2, so the case is clean. Dropping C1 → `main.js` is selected and reported; dropping C2 →
`base.js` is selected and reported; widening either conjunct (`.*/vitest-config/lib/.*\.js`) → both
reported.
`gritlint.json` (same as the standard block, with one line changed)
```json
{
  "packs": {
    "source-resolution": {
      "condition": "r\"['\\\"]@systemfsoftware/source['\\\"]\"",
      "presets": "r\"\\\"@systemfsoftware/tsconfig/.*\\\"\"",
      "presetFiles": "r\".*/tsconfig/(?:(?:bundler|tsc)/.*|node)\\.json\"",
      "sharedVitestConfig": "r\"['\\\"]@systemfsoftware/vitest-config['\\\"]\"",
      "sharedVitestConfigFile": "r\".*/vitest-config/lib/main\\.js\""
    }
  }
}
```
`packages/toolchain/vitest-config/lib/base.js`
```js
export const sourceCondition = '@systemfsoftware/source'

export const sharedConfig = {
  resolve: { conditions: [sourceCondition] },
}
```
`packages/toolchain/vitest-config/lib/main.js`
```js
export const sourceCondition = '@systemfsoftware/source'

export const sharedConfig = {
  ssr: { resolve: { conditions: [sourceCondition] } },
}
```

**B. `good/parameter-selects-decoy-name`** — kills GAP-AC: with
`sharedVitestConfigFile: r".*/vitest-config/lib/baseXjs"`, the rule's literal C1 rejects
`baseXjs`, so the broken file is clean today; with `base\.js` written unescaped it is selected and
reported. (`gritlint.json` identical to Proposal A with `base\\.js` → `baseXjs`.)
`packages/toolchain/vitest-config/lib/base.js`
```js
export const sourceCondition = '@systemfsoftware/source'

export const sharedConfig = {
  resolve: { conditions: [sourceCondition] },
  ssr: { resolve: { conditions: [sourceCondition] } },
}
```
`packages/toolchain/vitest-config/lib/baseXjs`
```js
export const sourceCondition = '@systemfsoftware/source'

export const sharedConfig = {
  resolve: { conditions: [sourceCondition] },
}
```

### Redundant cases

None. Each of the three existing cases kills a distinct clause (C3b+C3c, C3a+L1a-side, C3c+L1b);
`missing-ssr` is the only case that pins the file-alone fallback.

---

**Totals: 5 rules, 26 gaps (A–AD, of which GAP-AD is dead code with no closable case), 25 proposed cases.**