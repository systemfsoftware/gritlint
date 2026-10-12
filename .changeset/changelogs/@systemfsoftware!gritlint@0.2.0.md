# @systemfsoftware/gritlint@0.2.0

`cell-architecture/service-exports-no-layer` now reports a `*.service.ts` module that imports a driver (`node:*`, a Node built-in, `@effect/platform-*`, `@effect/sql-*`, `@effect/ai-*`, or a reviewed database client or vendor SDK) through a static import, a re-export, `import()` or `import x = require(...)`, and an import that renames a `*Layer` or `*Live` binding. A pure `static readonly layer` (`layerTest`, `layerConfig`) on the Service class now passes. Bumping gritlint can add findings for driver imports and remove findings for pure static layers; the `nextAction` says to move the driver import and its Layers into a module beside the service that only composition roots and tests import. Each reported violation now points at the offending node, every bundled rule names a reason code and a next action, and `--format json` is a versioned document. New formats: `concise`, `github` and `github-summary`. To upgrade:

1. Open each of your rule files with a `---` block declaring `reasonCode` (kebab-case) and `nextAction`; `locate` is optional and no other key is allowed, so drop any `title:` line. Otherwise the run exits 2.
2. Read JSON as `{ "schemaVersion": 1, ... }`: `rule` is now `ruleId`, `path` and `line` move into `location` (`path`, plus an optional `range`), and `files_selected` is now `filesSelected`. `gritlint schema --report` prints the schema.
3. In every bad fixture case, commit `expected-locations.txt`: one line per reported violation, `<ruleId> <path>[:<line>:<column>-<line>:<column>]`. gritlint now ships as an npm-style tarball that the flake builds as its `gritlint-tarball` output. A project installs it from the tarball file, with no registry and no network. The tarball carries static binaries for linux-x64 and linux-arm64 and a node launcher that runs the one for the host. On any other platform, `gritlint` exits 2 and names the two platforms it supports. When gritlint refuses a pack parameter name in its config, the error now gives the reason: the name is empty, or the name is not a GritQL identifier. Before this, every refused name, an empty one included, was reported as not being a GritQL identifier. Bundled rules report configs they used to miss, and stop false reports:

- Every bundled rule can check files at the repository root, so a single-package repository is no longer skipped. For `presetFiles` and `sharedVitestConfigFile`, start the value with `(?:.*/)?`, as the pack README shows, to include root files.
- `source-resolution/vitest-conditions` requires the source condition in both `resolve.conditions` and `ssr.resolve.conditions`, as its next action said.
- `source-resolution/tsconfig-paths` reports a `paths` key under the package's own name (`@acme/app/*`), not only the bare name.
- `vitest-conditions`, `tsconfig-condition` and `tsconfig-preset-module-mode` accept the source condition anywhere in a list (such as after `...defaultClientConditions`), and the last accepts module modes in any letter case.

Upgrading can surface new reports; fix each as its next action says. gritlint now parses a TypeScript object type or interface whose next member, on a new line, is a generic call signature (the shape Effect's `dual` overloads take), so such a module is judged instead of refusing the run. A target the engine cannot scan still refuses the run with exit 2, and stderr now carries one uncolored line with a reason code and a next action, the same in every format:

- `ERROR gritlint: <file>:<line>:<column>: target-does-not-parse: ...` when the file does not parse;
- `ERROR gritlint: <file>: target-too-large: ...` when the file is over the engine's 1,000,000-byte limit (before this, it was reported as a syntax error at 1:1);
- `ERROR gritlint: <file>: target-not-scanned: ...` for any other engine log on the file.

Stderr is colored only when it is a terminal. A consumer that matched the old `target file ... does not parse` message matches the reason code instead.
