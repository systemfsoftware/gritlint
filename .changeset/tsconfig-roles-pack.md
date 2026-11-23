---
"@systemfsoftware/gritlint": minor
---

gritlint gains the `tsconfig-roles` pack, which keeps tsconfig files to a closed set of roles.

- `tsconfig-file-name` is a path rule: a `tsconfig*.json` other than `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`, `tsconfig.test.json` or `tsconfig.build.json` fails, and `tsconfig.base.json` passes only at the repository root.
- `tsconfig-solution-file` fails a `tsconfig.json` that lacks `"files": []` or declares `include`.
- `tsconfig-reference-outside-package` fails a `tsconfig.json` reference with a `..` segment or an absolute path.
- `tsconfig-dom-with-runtime-types` fails one `tsconfig*.json` whose own `compilerOptions` lists `DOM` in `lib` and `node` or `@cloudflare/workers-types` in `types`.
- Four multifile rules join the files of one directory: `tsconfig-role-unreferenced` (a role file its `tsconfig.json` does not reference), `tsconfig-role-include-overlap` (the same `include` string in two role configs without `exclude`), and `tsconfig-dom-extends-runtime-types` and `tsconfig-runtime-types-extend-dom` (DOM and runtime types that meet through a same-directory `extends`). An `extends` that climbs to another directory or names a package is not followed.
