---
"@systemfsoftware/gritlint": minor
---

Bundled rules report configs they used to miss, and stop false reports:

- Every bundled rule can check files at the repository root, so a single-package repository is no longer skipped. For `presetFiles` and `sharedVitestConfigFile`, start the value with `(?:.*/)?`, as the pack README shows, to include root files.
- `source-resolution/vitest-conditions` requires the source condition in both `resolve.conditions` and `ssr.resolve.conditions`, as its next action said.
- `source-resolution/tsconfig-paths` reports a `paths` key under the package's own name (`@acme/app/*`), not only the bare name.
- `vitest-conditions`, `tsconfig-condition` and `tsconfig-preset-module-mode` accept the source condition anywhere in a list (such as after `...defaultClientConditions`), and the last accepts module modes in any letter case.

Upgrading can surface new reports; fix each as its next action says.
