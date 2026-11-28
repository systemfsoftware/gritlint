# A test project computes no declarations

## Problem

With `composite` or `declaration` on, a `tsconfig.test.json` computes a `.d.ts` for every module it compiles. `composite` does so even under `noEmit`, because it turns on declaration diagnostics. Test projects compile stories, fixtures and helpers, and an exported helper whose inferred type contains a large non-exported type (a router's route tree, for example) makes the declaration printer inline that type at every use site. The compiler can then exhaust memory before it reports TS4023 or TS7056. Nothing consumes a test project's declarations, so the cost buys nothing.

## Requirements

R1. `packs/tsconfig-roles` fails a `tsconfig.test.json` whose own `compilerOptions`, or a same-directory file it `extends` (one hop, as the pack's other `extends` rules do), sets `"composite": true`, `"declaration": true` or `"emitDeclarationOnly": true`.

R2. When a `tsconfig.test.json` extends a file outside its directory, which gritlint cannot follow, it must itself set `"composite": false`, `"declaration": false` and `"emitDeclarationOnly": false`. Otherwise it fails.

R3. The rule follows the pack's format: `rules/<name>.md` with `reasonCode`, `nextAction`, `locate`, the rule text, what the rule cannot see (presets reached through a package name, multi-hop `extends`), and a grit pattern; fixtures with good and bad cases and `expected-locations.txt`; and an entry in the README's rule list.

R4. `tsconfig-test-unreferenced` and `typecheck-build-mode` are unchanged. A non-composite test project can stay in its solution's `references`: `tsc -b` builds it, skips it when it is up to date, and reports its errors.

## Fixtures

- Bad: `composite` true; `declaration` true; `emitDeclarationOnly` true; `composite` true inherited from a same-directory `extends`; an external `extends` without the three explicit `false` values.
- Good: an external `extends` with all three `false` values; a test config with no `compilerOptions` and no `extends`.
- A file with comments is parsed and judged. A file with a trailing comma does
  not parse: the `json` grammar every rule in the pack shares rejects it, so
  gritlint reports `target-does-not-parse` and fails the run rather than passing
  the file silently; the fixtures carry comments only.

## Done when

The pack's fixtures and the repository's own test suite pass, and the PR is open.
