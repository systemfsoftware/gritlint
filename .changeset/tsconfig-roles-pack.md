---
"@systemfsoftware/gritlint": minor
---

gritlint gains the `tsconfig-roles` pack, which keeps tsconfig files to a closed set of roles.

- `tsconfig-file-name` is a path rule: a `tsconfig*.json` other than `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`, `tsconfig.test.json` or `tsconfig.build.json` fails, and `tsconfig.base.json` passes only at the repository root.
- `tsconfig-solution-file` fails a `tsconfig.json` that lacks `"files": []` or declares `include`.
- `tsconfig-reference-outside-package` fails a `tsconfig.json` reference with a `..` segment or an absolute path.
- `tsconfig-dom-with-runtime-types` fails one `compilerOptions` that lists `DOM` in `lib` and `node` or `@cloudflare/workers-types` in `types`. It does not resolve `extends`.
