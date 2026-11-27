---
"@systemfsoftware/gritlint": minor
---

A package can resolve to `dist/` for its workspace consumers by declaring `"gritlint": { "resolve": "dist" }` in its `package.json`. Three `source-resolution` rules check that its export map agrees:

- `dist-resolve-no-source-condition`: a declared package carries the source condition on no `exports` subpath.
- `tsdown-source-condition-required`: in a package with a bundler config and no declaration, every `exports` subpath that points at built output carries the condition.
- `gritlint-resolve-value`: a `"gritlint"` key is exactly `{ "resolve": "dist" }`.

`tsdown-exports` now accepts a bundler config that leaves `devExports` out; any `devExports` it sets must still be the condition string.
