---
"@systemfsoftware/gritlint": minor
---

Four bundled rules report configs they used to miss, and one stops a false report:

- `source-resolution/vitest-conditions` now requires the source condition in both `resolve.conditions` and `ssr.resolve.conditions`, as its next action always said. A config that sets only the ssr list, or names another condition in the client list, is now reported.
- `source-resolution/tsconfig-paths` now reports a `paths` key under the package's own name (`@acme/app/*`, `@acme/app/testing`), not only the bare name.
- `typecheck-build-mode` now checks a reference-only TypeScript project and its `typecheck` script when they sit at the repository root.
- `source-resolution/tsconfig-preset-module-mode` accepts module modes in any letter case, so `"module": "nodenext"` is no longer reported.

Upgrading can surface new reports in a repository that passed before; fix each one as its next action says.
