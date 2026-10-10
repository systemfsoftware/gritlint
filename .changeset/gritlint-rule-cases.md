---
"@systemfsoftware/gritlint": minor
---

Seven bundled rules report configs they used to miss, and one stops a false report:

- `source-resolution/vitest-conditions` requires the source condition in both `resolve.conditions` and `ssr.resolve.conditions`, as its next action said.
- `source-resolution/tsconfig-paths` reports a `paths` key under the package's own name (`@acme/app/*`), not only the bare name.
- `typecheck-build-mode` and the `source-resolution` rules `api-extractor-tsconfig`, `export-map-order`, `internals-export-map`, `publish-config-exports` and `tsconfig-paths` check a package at the repository root, so a single-package repository is no longer skipped.
- `source-resolution/tsconfig-preset-module-mode` accepts module modes in any letter case, so `"module": "nodenext"` is no longer reported.

Upgrading can surface new reports; fix each as its next action says.
