# @systemfsoftware/gritlint@0.7.0

A package can resolve to `dist/` for its workspace consumers by declaring `"gritlint": { "resolve": "dist" }` in its `package.json`. Three `source-resolution` rules check that its export map agrees:

- `dist-resolve-no-source-condition`: a declared package carries the source condition on no `exports` subpath.
- `tsdown-source-condition-required`: in a package with a bundler config and no declaration, every `exports` subpath that points at built output carries the condition.
- `gritlint-resolve-value`: a `"gritlint"` key is exactly `{ "resolve": "dist" }`.

`tsdown-exports` now accepts a bundler config that leaves `devExports` out; any `devExports` it sets must still be the condition string.

`source-resolution/tsconfig-condition` reports the tsconfig file that fails it. A project with no `customConditions` key was reported against the alphabetically first tsconfig in its directory, which could be a sibling that passes. The finding now points at the failing project's `include`. A project whose `customConditions` lacks the condition is still reported at that list.

**Breaking:** the `build-output-imports` pack is removed. A `gritlint.json` that still lists it fails to load; delete `"build-output-imports"` from `packs` when you upgrade.

`ts-extensions` now reads only tsconfig files: its source-scanning `ts-extension-specifier` rule is removed. With `allowImportingTsExtensions` and `rewriteRelativeImportExtensions` off, `tsc` rejects a `.ts` specifier in an `import`, `export ... from` or `import()` of a file it checks (TS5097). It does not reject one in a type-position `import("./x.ts")` type, and it does not see a file outside every tsconfig project; nothing catches those now.

On a 4,600-source monorepo a scan went from 274 to 22 CPU-seconds (14.6 s to 1.8 s wall).
