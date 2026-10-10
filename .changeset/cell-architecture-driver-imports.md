---
"@systemfsoftware/gritlint": minor
---

`cell-architecture/service-exports-no-layer` now reports a `*.service.ts` module that imports a driver (`node:*`, a Node built-in, `@effect/platform-*`, `@effect/sql-*`, `@effect/ai-*`, or a reviewed database client or vendor SDK) through a static import, a re-export, `import()` or `import x = require(...)`, and an import that renames a `*Layer` or `*Live` binding. A pure `static readonly layer` (`layerTest`, `layerConfig`) on the Service class now passes. Bumping gritlint can add findings for driver imports and remove findings for pure static layers; the `nextAction` says to move the driver import and its Layers into a module beside the service that only composition roots and tests import.
