# @systemfsoftware/gritlint@0.4.0

Path rules now judge hidden files, and a path rule no longer breaks a GritQL rule beside it.

- The walk reaches dot-named files and folders (`.DS_Store`, `.hidden/`), so a path rule judges them. No GritQL rule reads them, as before.
- A path rule's block was handed to every GritQL rule in the same scope as a library pattern, so a local path rule next to a local GritQL rule failed to compile. Path rules are no longer library patterns.

gritlint gains path rules, which judge file paths and never read a file.

- A rule whose fenced block is `path` takes `scope <regex>` (at most once) and `allow <regex>` lines. A path in scope that matches no `allow` pattern is one finding at that path. Patterns match the whole relative path, written with `/` separators.
- `gritlint check` now walks every non-ignored file whatever its extension, so a `.txt` file or a file with no extension is judged by a path rule. A file no rule language parses is selected as unparsed, is never read, and reaches no GritQL rule.
- A path rule takes no `locate` key and does not export to Biome (`export-path-only`).
- The files-selected count in a report now includes every walked file.

gritlint gains the `ts-extensions` pack, which fails TypeScript file extensions in module specifiers.

- `tsconfig-ts-extensions` fails a `compilerOptions` that sets `allowImportingTsExtensions` or `rewriteRelativeImportExtensions` to `true`.
- `ts-extension-specifier` fails a relative, absolute, `#` subpath or package subpath specifier that ends in `.ts`, `.tsx`, `.mts` or `.cts`, in a static import, an `export ... from`, a dynamic `import()` or an `import x = require()`. Strings that are not module specifiers are not judged.
- Both are single-file rules, so they also export to Biome.

It also gains the `build-output-imports` pack. Its `dist-import-specifier` rule fails a relative specifier that climbs out of the importing directory into another directory's `dist` segment, an absolute specifier with a `dist` segment, and a package specifier whose subpath has a `dist` segment. It is a single-file rule and exports to Biome.

gritlint gains the `tsconfig-roles` pack, which keeps tsconfig files to a closed set of roles.

- `tsconfig-file-name` is a path rule: a `tsconfig*.json` other than `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`, `tsconfig.test.json` or `tsconfig.build.json` fails, and `tsconfig.base.json` passes only at the repository root.
- `tsconfig-solution-file` fails a `tsconfig.json` that lacks `"files": []` or declares `include`.
- `tsconfig-reference-outside-package` fails a `tsconfig.json` reference with a `..` segment or an absolute path.
- `tsconfig-dom-with-runtime-types` fails one `tsconfig*.json` whose own `compilerOptions` lists `DOM` in `lib` and `node` or `@cloudflare/workers-types` in `types`.
- Four multifile rules join the files of one directory: `tsconfig-role-unreferenced` (a role file its `tsconfig.json` does not reference), `tsconfig-role-include-overlap` (the same `include` string in two role configs without `exclude`), and `tsconfig-dom-extends-runtime-types` and `tsconfig-runtime-types-extend-dom` (DOM and runtime types that meet through a same-directory `extends`). An `extends` that climbs to another directory or names a package is not followed.
