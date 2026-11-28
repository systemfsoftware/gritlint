---
"@systemfsoftware/gritlint": minor
---

The `tsconfig-roles` pack adds `tsconfig-test-declarations`: a `tsconfig.test.json` must not set `composite`, `declaration` or `emitDeclarationOnly` to `true`, and must set all three to `false` itself when it `extends` a file outside its directory, which gritlint cannot read.

With any of the three on, a test project computes a `.d.ts` for every module it compiles; `composite` does so even under `noEmit`. An exported test helper whose inferred type contains a large non-exported type makes the declaration printer inline it at every use site, and the compiler can exhaust memory before it reports TS4023 or TS7056. Nothing consumes a test project's declarations. A non-composite test project can stay in its solution's `references`, so `tsconfig-test-unreferenced` is unchanged.
