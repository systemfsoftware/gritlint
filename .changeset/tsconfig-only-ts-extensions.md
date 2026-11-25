---
"@systemfsoftware/gritlint": minor
---

**Breaking:** the `build-output-imports` pack is removed. A `gritlint.json` that still lists it fails to load; delete `"build-output-imports"` from `packs` when you upgrade.

`ts-extensions` now reads only tsconfig files: its source-scanning `ts-extension-specifier` rule is removed. With `allowImportingTsExtensions` and `rewriteRelativeImportExtensions` off, `tsc` rejects a `.ts` specifier in an `import`, `export ... from` or `import()` of a file it checks (TS5097). It does not reject one in a type-position `import("./x.ts")` type, and it does not see a file outside every tsconfig project; nothing catches those now.

On a 4,600-source monorepo a scan went from 274 to 22 CPU-seconds (14.6 s to 1.8 s wall).
