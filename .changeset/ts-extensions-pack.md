---
"@systemfsoftware/gritlint": minor
---

gritlint gains the `ts-extensions` pack, which fails TypeScript file extensions in module specifiers.

- `tsconfig-ts-extensions` fails a `compilerOptions` that sets `allowImportingTsExtensions` or `rewriteRelativeImportExtensions` to `true`.
- `ts-extension-specifier` fails a relative, absolute, `#` subpath or package subpath specifier that ends in `.ts`, `.tsx`, `.mts` or `.cts`, in a static import, an `export ... from`, a dynamic `import()` or an `import x = require()`. Strings that are not module specifiers are not judged.
- Both are single-file rules, so they also export to Biome.

It also gains the `build-output-imports` pack. Its `dist-import-specifier` rule fails a relative specifier that climbs out of the importing directory into another directory's `dist` segment, an absolute specifier with a `dist` segment, and a package specifier whose subpath has a `dist` segment. It is a single-file rule and exports to Biome.
