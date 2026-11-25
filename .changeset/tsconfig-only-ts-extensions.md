---
"@systemfsoftware/gritlint": minor
---

`ts-extensions` reads only tsconfig files: its `ts-extension-specifier` rule, which scanned every JavaScript and TypeScript source, is removed, since with `allowImportingTsExtensions` and `rewriteRelativeImportExtensions` off the compiler rejects a `.ts` specifier itself (TS5097). The `build-output-imports` pack is removed; a config that still lists it fails to load, so delete it from `packs`. Together the two rules were about 97% of a scan's CPU time on a 4,600-source monorepo.
