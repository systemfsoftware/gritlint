# Gap analysis: `source-resolution` (5 rules)

Read-only analysis per `local://pr5-gap-method.md`. Sources: `packs/source-resolution/README.md`, `pack.json`, the five `rules/*.md`, and every file under `packs/source-resolution/fixtures/{tsconfig-condition,tsconfig-paths,tsconfig-preset-module-mode,tsdown-exports,vitest-conditions}/`.

Range convention confirmed from existing cases: **end column exclusive**. `tsdown-exports/bad/missing-custom-exports` expects `2:3-2:53` for `exports: { devExports: '@systemfsoftware/source' }` (50 chars from col 3). `tsconfig-condition/bad/other-condition` expects `5:5-5:42` for `"customConditions": ["@other/source"]` (37 chars from col 5). The same shape recurs in `tsconfig-preset-module-mode/bad/cleared-condition` (`4:5-4:27` = 22 chars). A case whose `locate` fails (e.g. `tsconfig-condition/bad/no-condition-no-preset`, `vitest-conditions/bad/only-client-conditions`, `tsconfig-preset-module-mode/bad/no-module-mode`) asserts a path-only line.

`gritlint.json` is byte-identical in every case of these five rules (the five README parameters). Where a proposed case needs a different parameter it says so explicitly.

---

## source-resolution/tsconfig-condition

### Clauses

| id | text |
|----|------|
| C1 | `$name <: r".*/tsconfig[^/]*\.json"` (project files only, never `tsconfig/<preset>.json`) |
| C2 | `$program <: contains `"include": [$first, $...]`` — a **non-empty** include |
| C3 | `$program <: not contains `"extends": $extends` where { … }` |
| C3a | `$extends <: presets()` |
| C3b | `$extends <: contains presets()` |
| C3c | `$extends <: includes "./"` |
| C4 | `$program <: not contains `"customConditions": [$cond, $...]` where { `$cond <: condition()` }` |
| L1 | locate `"customConditions": $list` where { `$list <: or { `[]`, `[$cond, $...]` where { `$cond <: not condition()` } }` } |

### Cases

- `bad/no-condition-no-preset` — `extends: "@other/base"`, no `customConditions`; asserts a **path-only** location, so it also pins that locate may fail.
- `bad/other-condition` — `extends: "@other/base"`, `customConditions: ["@other/source"]`; asserts `5:5-5:42` (L1b).
- `good/names-condition` — project names the condition itself.
- `good/extends-preset` — `extends` exactly matches `presets()`.
- `good/extends-relative` — `tsconfig.build.json` extends `"./tsconfig.app.json"` (C3c).
- `good/solution-root` — `files: []` + `references`, no `include`.

### Coverage

| clause | killing case |
|--------|--------------|
| C1 narrowed to `tsconfig\.json` | `bad/other-condition` (uses `tsconfig.test.json`) |
| C1 widened `[^/]*`->`.*` | **GAP 1** |
| C2 dropped | `good/solution-root` starts reporting |
| C2 widened `[$first,$...]`->`$_` | **GAP 2** |
| C3 `not` flipped | `bad/no-condition-no-preset`, `bad/other-condition` go clean |
| C3a dropped | `good/extends-preset` |
| C3b dropped | **GAP 3** |
| C3c dropped | `good/extends-relative` |
| C4 `not` flipped / whole clause dropped | `bad/no-condition-no-preset`, `bad/other-condition` go clean |
| C4's `$cond <: condition()` dropped | `bad/other-condition` goes clean |
| C4 `[$cond,$...]` widened to `$_` | **GAP 4** |
| L1a `[]` branch | **GAP 4** |
| L1b `not condition()` | `bad/other-condition` |
| locate matching the wrong node / always yielding a range | `bad/no-condition-no-preset` (path-only) |

### Proposed cases

**GAP 1 — good `nested-tsconfig-file`** (a file under `tsconfig/` is a preset, not a project)
`fixtures/tsconfig-condition/good/nested-tsconfig-file/packages/app/tsconfig/base.json`

```json
{
  "include": ["src"],
  "compilerOptions": {
    "module": "preserve"
  }
}
```
Correct rule: `[^/]*` cannot span `/`, so the file is never named -> clean. Widening `[^/]*` to `.*` names it, and C2/C3/C4 all pass -> finding. Also guards against a future `.*/tsconfig\.json` generalization. `good/extends-preset` alone cannot catch this: it lives under `packages/app/`, not `tsconfig/`.

**GAP 2 — good `empty-include`**
`fixtures/tsconfig-condition/good/empty-include/packages/app/tsconfig.app.json`

```json
{
  "extends": "@other/base",
  "include": []
}
```
Correct rule: `[$first, $...]` needs one element -> clean. Widening to `$_` reports it.

**GAP 3 — good `extends-preset-prefix`** (kills the `contains presets()` branch only, via a narrowed `presets`)
`gritlint.json` — same as siblings except `"presets": "r\"\\\"@systemfsoftware/tsconfig/base\\\"\""`.
`packages/app/tsconfig.app.json`

```json
{
  "extends": "@systemfsoftware/tsconfig/base/no-dom",
  "include": ["src"]
}
```
Correct rule: `presets()` fails (regex must match the whole node), `contains presets()` succeeds -> clean. Dropping C3b reports it. With the shipped `.*`-suffixed parameter C3b is unreachable, which is why the parameter must be narrowed here.

**GAP 4 — bad `cleared-conditions`** (a project that empties the list; also pins L1a and the non-emptiness of `[$cond, $...]`)
`fixtures/tsconfig-condition/bad/cleared-conditions/packages/app/tsconfig.test.json`

```json
{
  "extends": "@other/base",
  "compilerOptions": {
    "customConditions": []
  },
  "include": ["tests"]
}
```
`expected-locations.txt`

```
source-resolution/tsconfig-condition packages/app/tsconfig.test.json:4:5-4:27
```
(`"customConditions": []` is 22 chars from col 5 on line 4; same shape as `tsconfig-preset-module-mode/bad/cleared-condition`.) Widening C4 to `$_` would match `[]`, veto the report, and the case would go clean.

### Redundant

`good/extends-relative`'s `tsconfig.app.json` is inert — the rule never resolves `./`, so deleting it leaves the case's verdict unchanged. Keep for readability; not a failing-case redundancy.

---

## source-resolution/tsconfig-paths

### Clauses

| id | text |
|----|------|
| P1 | `$name <: r"(.*/)package\.json"($dir)` (capture the directory) |
| P2 | `$program <: contains `"name": $declared`` |
| P3 | `$declared <: r"\"(.*)\""($package)` (strip the JSON quotes) |
| P4 | `$name <: r"(.*/)tsconfig[^/]*\.json"($config_dir)` |
| P5 | `$config_dir <: $dir` — the tsconfig must sit in the package's own directory |
| P6 | `$program <: contains `"paths": $paths`` |
| P7 | `$paths <: contains `$mapped: $target`` |
| P8 | `$mapped <: r"\"(.*)\""($key)` |
| P9 | `$key <: $package` — exact name equality |
| L1 | locate `"paths": $_` |

### Cases

- `bad/maps-own-name` — `@acme/app` mapped to `./src/index.ts` in `tsconfig.json`; asserts `3:5-5:6` (L1, whole object).
- `good/no-self-paths` — only `@acme/*` is mapped.

### Coverage

| clause | killing case |
|--------|--------------|
| P1/P2/P3 dropped or capture lost | `bad/maps-own-name` goes clean |
| P3/P8 asymmetric quote-stripping (one side keeps `"`) | `bad/maps-own-name` goes clean |
| P4 narrowed to `tsconfig\.json` | **GAP 5** |
| P4 widened `[^/]*`->`.*` | masked by P5 (see note) |
| P5 dropped | **GAP 6** |
| P6 dropped / P7 widened to `$_` | `bad/maps-own-name` goes clean |
| P7's `$target` conjunct | **unkillable** |
| P8 widened (dropped quotes on both sides) | not killed — see note |
| P9 loosened (e.g. `contains`) | `good/no-self-paths` |
| P9 vs subpath wildcards `@acme/app/*` | **GAP 7 (needs an intent ruling)** |
| L1 pointing at the wrong node | `bad/maps-own-name`'s asserted `3:5-5:6` |

Notes. P8 with the quotes stripped on **both** sides (`r"(.*)"` on P3 and P8) is invisible to both cases: both sides then carry quotes and stay equal. That is a real weakening nobody can catch with a fixture; the asymmetry case above is what is actually covered. P7's `$target` is unkillable — every JSON key has a value, so `contains `$mapped: $target`` and `contains `$mapped:`` accept the same texts; `$target` is inert syntax. P4 widening is untestable, not untested: a widened `.*` would also need P5 to be wrong to produce a finding.

### Proposed cases

**GAP 5 — bad `build-config-maps-own-name`** (P4 must accept `tsconfig*.json`, not only `tsconfig.json`)
`fixtures/tsconfig-paths/bad/build-config-maps-own-name/packages/app/package.json`

```json
{
  "name": "@acme/app"
}
```
`packages/app/tsconfig.build.json`

```json
{
  "compilerOptions": {
    "paths": {
      "@acme/app": ["./src/index.ts"]
    }
  }
}
```
`expected-locations.txt`

```
source-resolution/tsconfig-paths packages/app/tsconfig.build.json:3:5-5:6
```

**GAP 6 — good `tsconfig-in-other-directory`** (P5's same-directory join)
`fixtures/tsconfig-paths/good/tsconfig-in-other-directory/packages/app/package.json`

```json
{
  "name": "@acme/app"
}
```
`packages/lib/tsconfig.json`

```json
{
  "compilerOptions": {
    "paths": {
      "@acme/app": ["./src/index.ts"]
    }
  }
}
```
Correct rule: `packages/lib` != `packages/app` -> clean. Dropping P5 joins them and reports at `3:5-5:6`.

**GAP 7 — good `subpath-wildcard`** (conditional on an intent ruling)
`fixtures/tsconfig-paths/good/subpath-wildcard/packages/app/package.json`

```json
{
  "name": "@acme/app"
}
```
`packages/app/tsconfig.json`

```json
{
  "compilerOptions": {
    "paths": {
      "@acme/app/*": ["./src/*"]
    }
  }
}
```
As a **good** case it kills a loosened P9 (`contains`, prefix). As a **bad** case it documents that `paths` cannot express the subpath form at all — the rule's prose says "the package's own published name", and `@acme/app/*` is that name's subpath space. If the maintainers intend to report it, the rule needs a wildcard-aware comparison and this case belongs under `bad/` with `expected-locations.txt` = `source-resolution/tsconfig-paths packages/app/tsconfig.json:3:5-5:6`. Do not add either until that is ruled on.

### Redundant

None. Both cases are load-bearing for P9/P1–P3.

---

## source-resolution/tsconfig-preset-module-mode

### Clauses

| id | text |
|----|------|
| C1 | `$name <: r".*/tsconfig/.*\.json"` |
| C2 | `$name <: presetFiles()` |
| C3a | `not contains `"module": $mode` where { `$mode <: r"\"(?:[Nn]odeNext|[Nn]ode16|[Pp]reserve)\""` }` |
| C3b | `not contains `"moduleResolution": $resolution` where { `$resolution <: r"\"(?:[Bb]undler|[Nn]ode16|[Nn]odeNext)\""` }` |
| C4 | `contains `"customConditions": []`` |
| C5 | `contains `"customConditions": [$cond, $...]` where { `$cond <: not condition()` }` |
| — | outer `or { and { C3a, C3b }, C4, C5 }` |
| L1 | locate `"customConditions": $list` where { `[]` or wrong condition } |
| L2 | locate `"module": $mode` where { `$mode <: not r"\"(?:[Nn]odeNext|[Nn]ode16|[Pp]reserve)\""`, `$program <: not contains `"moduleResolution": $resolution` where { ok resolution } }` |

### Cases

- `bad/no-module-mode` — no `module`, no `moduleResolution`; **path-only** location, pinning that L2 fails when no `module` key exists.
- `bad/cleared-condition` — `module: "preserve"`, `customConditions: []`; asserts `4:5-4:27` (L1a).
- `good/condition-and-mode` — both correct.
- `good/module-mode-only` — `module: "preserve"`, no `customConditions`.

### Coverage

| clause | killing case |
|--------|--------------|
| C1/C2 dropped | all three preset cases |
| C3a `not contains` flipped to `contains` | `good/module-mode-only`, `good/condition-and-mode` start reporting |
| C3b dropped | **GAP 8** |
| C3a alternatives `NodeNext` / `Node16` / lower case | **GAP 8** |
| C3b alternatives `Bundler` / `Node16` / `NodeNext` | **GAP 8** |
| C1 narrowed to `tsconfig/bundler/.*` | **GAP 9** |
| C1 widened | masked by C2 (see note) |
| C4 dropped | `bad/cleared-condition` |
| C4 widened to `$_` | `good/condition-and-mode` |
| C5 dropped or `not` flipped | `good/condition-and-mode` |
| C5 `[$cond,$...]` widened to `$_` | `bad/cleared-condition` and `good/condition-and-mode` both flip |
| L1a | `bad/cleared-condition` |
| L1b (preset naming a different condition) | **GAP 10** |
| L2 positive path (`"module"` present and wrong) | **GAP 10** |
| L2's `not contains ok moduleResolution` conjunct | **unreachable** |
| locate matching the wrong node | `bad/cleared-condition`'s asserted range; `bad/no-module-mode`'s path-only line |

Notes. C1 is strictly broader than C2 (`presetFiles` is `.*/tsconfig/(?:(?:bundler|tsc)/.*|node)\.json`), so C1 *widening* can never produce a new finding and is untestable; only C1 narrowing is testable. L2's second conjunct is dead code: the rule matches the no-module-mode branch only when no acceptable `moduleResolution` exists, and when C4/C5 is the trigger the locate is already satisfied by L1 — so no fixture can distinguish it from a dropped conjunct.

### Proposed cases

**GAP 8 — good `module-mode-alternatives`** (every regex alternative of C3a and C3b, and C3b as a branch; multi-file so one case covers all six)
`fixtures/tsconfig-preset-module-mode/good/module-mode-alternatives/`

`packages/toolchain/tsconfig/bundler/next.json`

```json
{
  "compilerOptions": {
    "module": "NodeNext"
  }
}
```
`packages/toolchain/tsconfig/bundler/n16.json`

```json
{
  "compilerOptions": {
    "module": "Node16"
  }
}
```
`packages/toolchain/tsconfig/bundler/lower.json`

```json
{
  "compilerOptions": {
    "module": "nodenext"
  }
}
```
`packages/toolchain/tsconfig/node.json`

```json
{
  "compilerOptions": {
    "moduleResolution": "Bundler"
  }
}
```
`packages/toolchain/tsconfig/tsc/res-n16.json`

```json
{
  "compilerOptions": {
    "moduleResolution": "Node16"
  }
}
```
`packages/toolchain/tsconfig/tsc/res-next.json`

```json
{
  "compilerOptions": {
    "moduleResolution": "NodeNext"
  }
}
```
No `expected-locations.txt` (good). Kills: dropping C3b (the three resolution files have no `module`, so C3a alone would report); dropping `NodeNext` from C3a (`next.json`); dropping `Node16` from C3a (`n16.json`); dropping the lower-case class from C3a (`lower.json`); dropping any single alternative from C3b. Also gives `presetFiles` a `good`-side reading of its `|node` and `tsc` alternatives.

**GAP 9 — bad `preset-under-tsc-and-node`** (C1 must accept every shape `presetFiles` accepts)
`fixtures/tsconfig-preset-module-mode/bad/preset-under-tsc-and-node/packages/toolchain/tsconfig/tsc/lib.json`

```json
{
  "compilerOptions": {
    "module": "preserve",
    "customConditions": []
  }
}
```
`packages/toolchain/tsconfig/node.json` — identical contents.
`expected-locations.txt`

```
source-resolution/tsconfig-preset-module-mode packages/toolchain/tsconfig/node.json:4:5-4:27
source-resolution/tsconfig-preset-module-mode packages/toolchain/tsconfig/tsc/lib.json:4:5-4:27
```
Narrowing C1 to `.*/tsconfig/bundler/.*\.json` (or to any single `bundler/` file) makes both files invisible -> the case goes clean. Every existing preset case sits under `tsconfig/bundler/`, so nothing currently pins this.

**GAP 10 — bad `other-condition`** and **bad `node10-module`** (C5 + locate L1b; C3a's negative + locate L2)
`fixtures/tsconfig-preset-module-mode/bad/other-condition/packages/toolchain/tsconfig/bundler/dom.json`

```json
{
  "compilerOptions": {
    "module": "preserve",
    "customConditions": ["@other/source"]
  }
}
```
`expected-locations.txt`

```
source-resolution/tsconfig-preset-module-mode packages/toolchain/tsconfig/bundler/dom.json:4:5-4:42
```
(line 4, 37 chars from col 5 — the same span `tsconfig-condition/bad/other-condition` asserts.) C5 is otherwise untested: both bad cases either lack the key or empty it.

`fixtures/tsconfig-preset-module-mode/bad/node10-module/packages/toolchain/tsconfig/bundler/dom.json`

```json
{
  "compilerOptions": {
    "module": "node10"
  }
}
```
`expected-locations.txt`

```
source-resolution/tsconfig-preset-module-mode packages/toolchain/tsconfig/bundler/dom.json:3:5-3:23
```
(`"module": "node10"` is 18 chars from col 5.) `bad/no-module-mode` omits the key entirely, so locate L2 never gets a positive test; this case also pins L2's own `not` conjunct.

### Redundant

None of the four cases is redundant. `good/module-mode-only` looks subsumed by `good/condition-and-mode` but is the only case isolating "preset with no `customConditions` key" from "preset naming the condition".

---

## source-resolution/tsdown-exports

### Clauses

| id | text |
|----|------|
| D1 | `$name <: r".*/tsdown\.config\.ts"` |
| D2 | `contains `exports: $exports`` |
| D3 | `$exports <: `{ $first, $... }`` (inline object) |
| D4a | `not contains `customExports: $_`` |
| D4b | `not contains `devExports: $_`` |
| D4c | `contains `devExports: $value` where { `$value <: not condition()` }` |
| D6 | `contains `clean: false`` |
| L1 | locate `clean: false` |
| L2 | locate `devExports: $value` where { `$value <: not condition()` }` |
| L3 | locate `exports: $exports` where { `{ $first, $... }`, or { not contains `customExports: $_`, not contains `devExports: $_` } }` |

### Cases

- `bad/clean-false` — fully configured plus `clean: false`; asserts `3:3-3:15` (L1).
- `bad/dev-exports-true` — `devExports: true`; asserts `2:14-2:30` (L2).
- `bad/missing-custom-exports` — `exports: { devExports: '@systemfsoftware/source' }`; asserts `2:3-2:53` (L3 via D4a).
- `good/named-condition` — both keys, string condition.

### Coverage

| clause | killing case |
|--------|--------------|
| D1 dropped or widened to all `*.ts` | **GAP 11** |
| D2 dropped | `bad/missing-custom-exports`, `bad/dev-exports-true` go clean |
| D3 dropped / widened to `$_` | **GAP 12** |
| D4a dropped or `not` flipped | `bad/missing-custom-exports` |
| D4b dropped or `not` flipped | **GAP 13** |
| D4c dropped / `not` flipped to `condition()` | `bad/dev-exports-true` |
| D4c on a *wrong string* (not `true`) | **GAP 14** |
| outer `or` -> `and` | `bad/clean-false` (both arms true) |
| inner `or` -> `and` | `bad/missing-custom-exports` |
| D6 widening `clean: $_` / `clean: $v` | **GAP 15** |
| L1 / L2 / L3 wrong node | the three asserted ranges |
| `exports: $exports` as a whole-object widen in L3 | `bad/missing-custom-exports` |

### Proposed cases

**GAP 11 — good `other-bundler-config-ignored`**
`fixtures/tsdown-exports/good/other-bundler-config-ignored/packages/app/tsup.config.ts`

```ts
export default defineConfig({
  clean: false,
  exports: { devExports: true },
})
```
Correct rule: D1 does not name the file -> clean. Any widening of D1 (or dropping it) reports both `clean: false` and a boolean `devExports`.

**GAP 12 — good `exports-from-identifier`** (D3 requires an inline object)
`fixtures/tsdown-exports/good/exports-from-identifier/packages/app/tsdown.config.ts`

```ts
import { exports as sharedExports } from '@systemfsoftware/preset'

export default defineConfig({
  exports: sharedExports,
})
```
Correct rule: D3 fails -> clean. Dropping D3 lets `$exports` bind the identifier, and `not contains `customExports: $_`` then holds -> finding. Needs the same intent ruling as GAP 7: if an imported `exports` is meant to be a finding, D3 should be deleted from the rule and this case moved to `bad/`.

**GAP 13 — bad `missing-dev-exports`** (D4b, and locate L3's second branch)
`fixtures/tsdown-exports/bad/missing-dev-exports/packages/app/tsdown.config.ts`

```ts
export default defineConfig({
  exports: { customExports: injectTypes },
})
```
`expected-locations.txt`

```
source-resolution/tsdown-exports packages/app/tsdown.config.ts:2:3-2:42
```
(`exports: { customExports: injectTypes }` is 39 chars from col 3 — same convention as the existing `2:3-2:53`.)

**GAP 14 — bad `dev-exports-other-condition`** (optional; the `condition()` wiring on a string value)
`fixtures/tsdown-exports/bad/dev-exports-other-condition/packages/app/tsdown.config.ts`

```ts
export default defineConfig({
  exports: { devExports: '@other/source', customExports: injectTypes },
})
```
`expected-locations.txt`

```
source-resolution/tsdown-exports packages/app/tsdown.config.ts:2:14-2:43
```
(`devExports: '@other/source'` is 29 chars from col 14 — `bad/dev-exports-true` asserts the same start col for the 16-char `devExports: true`.) `dev-exports-true` already kills a dropped `not`, so this is defence in depth for the parameter itself rather than a structural clause.

**GAP 15 — good `clean-true`**
`fixtures/tsdown-exports/good/clean-true/packages/app/tsdown.config.ts`

```ts
export default defineConfig({
  exports: { devExports: '@systemfsoftware/source', customExports: injectTypes },
  clean: true,
})
```
Kills any widening of `clean: false` (`clean: $_`, `clean: $v`, dropping the literal).

### Redundant

`good/named-condition` is load-bearing (a dropped `not` in D4b/D4c, a `$_` in D4a/D4b, or an `or`->`and` slip all turn it into a finding). None redundant.

---

## source-resolution/vitest-conditions

### Clauses

| id | text |
|----|------|
| V1 | `$name <: r".*/vitest\.config\.(?:ts|mts)"` |
| V2 | `$program <: not contains sharedVitestConfig()` |
| V3 | `$program <: not contains `ssr: { resolve: { conditions: [$cond, $...] } }` where { `$cond <: condition()` }` |
| L1 | locate `ssr: $ssr` where { `$ssr <: not contains `conditions: [$cond, $...]` where { `$cond <: condition()` } }` |

### Cases

- `bad/only-client-conditions` — client `resolve.conditions` only; **path-only** location, pinning that locate may fail.
- `bad/ssr-other-condition` — ssr list names `@other/source`; asserts `5:3-5:54` (L1).
- `good/both-conditions` — both lists name the condition.
- `good/imports-shared` — imports `@systemfsoftware/vitest-config` (single quotes).

### Coverage

| clause | killing case |
|--------|--------------|
| V1 dropped / widened to all configs | **GAP 16** |
| V1's `|mts` alternative dropped | **GAP 17** |
| V2 dropped or `not` flipped | `good/imports-shared` |
| V2's parameter quote class `['\"]` | **GAP 18** |
| V3 dropped / `not` flipped | `good/both-conditions` |
| V3's `$cond <: condition()` dropped | `bad/ssr-other-condition` goes clean |
| V3's literal `ssr: { resolve: { … } }` shape widened | `good/both-conditions` |
| L1 wrong node / `not` dropped | `bad/ssr-other-condition`'s asserted `5:3-5:54` |
| locate always producing a range | `bad/only-client-conditions` (path-only) |
| client `resolve.conditions` requirement | **not in the rule at all — see finding** |

### Proposed cases

**GAP 16 — good `non-vitest-config-ignored`**
`fixtures/vitest-conditions/good/non-vitest-config-ignored/packages/app/vite.config.ts`

```ts
export default {
  resolve: { conditions: ['@systemfsoftware/source'] },
}
```
Correct rule: V1 does not name `vite.config.ts` -> clean. Any V1 widening reports it, since it has no `ssr` key.

**GAP 17 — bad `mts-config`** (V1 must accept `.mts`)
`fixtures/vitest-conditions/bad/mts-config/packages/app/vitest.config.mts`

```ts
import { defineConfig } from 'vite'

export default defineConfig({
  resolve: { conditions: ['@systemfsoftware/source'] },
})
```
`expected-locations.txt`

```
source-resolution/vitest-conditions packages/app/vitest.config.mts
```
Path-only: no `ssr` key, so locate fails. Narrowing V1 to `vitest\.config\.ts` makes the case go clean.

**GAP 18 — good `imports-shared-double-quoted`** (the parameter is quote-aware by design)
`fixtures/vitest-conditions/good/imports-shared-double-quoted/packages/app/vitest.config.ts`

```ts
import { defineConfig, sharedConfig } from "@systemfsoftware/vitest-config"

export default defineConfig({ ...sharedConfig })
```
Kills narrowing `sharedVitestConfig` to `'`-only in the parameter; `good/imports-shared` uses single quotes only.

### Redundant

None. `bad/only-client-conditions` and `bad/ssr-other-condition` differ only in the ssr list, and that difference is exactly what pins V3's `condition()` conjunct — keep both.

---

## Cross-rule findings (not fixture gaps, but they decide two of the proposals above)

1. **`vitest-conditions` prose claims more than the code checks.** The rule's `nextAction` says "Import the shared `@systemfsoftware/vitest-config`, or set the source condition on both `resolve.conditions` and `ssr.resolve.conditions`", and the README table says "sets both". The GritQL block only tests the ssr list (V3): a config with `ssr: { resolve: { conditions: ['@systemfsoftware/source'] } }` and **no** client `resolve.conditions` produces no finding. Every existing vitest fixture sets both keys, so the suite cannot distinguish the shipped behaviour from the documented one. Needs a ruling before any fixture is added: either add a bad case (and fix V3 to also require the client list) or amend the `nextAction` and README.
2. **`tsconfig-paths` and `tsdown-exports` both hinge on unresolved intent** for values they cannot inspect (`@acme/app/*` subpath mappings; `exports: someIdentifier`). The proposed GAP 7 and GAP 12 cases are deliberately listed with both outcomes; neither should be committed before the ruling.