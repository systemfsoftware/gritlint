---
reasonCode: tsconfig-role-file-unknown
nextAction: Rename the file to one of tsconfig.json, tsconfig.app.json, tsconfig.node.json, tsconfig.test.json or tsconfig.build.json (tsconfig.base.json only at the repository root), or fold its settings into the role it belongs to.
---

# A tsconfig file is one of the closed set of roles

Every `tsconfig*.json` is `tsconfig.json`, `tsconfig.app.json`,
`tsconfig.node.json`, `tsconfig.test.json` or `tsconfig.build.json`; the shared
base `tsconfig.base.json` exists only at the repository root.

A role file exists for one distinct mode: a `lib`, `types` or emit setting that
no other file in the package shares. A file named for a feature, a folder or a
one-off (`tsconfig.page.json`, `tsconfig.worker.json`, `tsconfig.bin.json`)
invents a role without a mode behind it. Each invented role is one more project
for `tsc -b` to schedule, one more place whose `lib` and `types` can drift from
its siblings, and a file that no tool outside the repository knows to look for.

Fix it by deciding which role the file is. If it differs from a role only in
`include`, widen that role's `include`. If it needs its own `lib`, `types` or
emit mode, it is that mode's role: `app` (DOM), `node` (Node or Worker runtime),
`test`, or `build` (emit).

```path
scope (?:.*/)?tsconfig[^/]*\.json
allow (?:.*/)?tsconfig(?:\.(?:app|node|test|build))?\.json
allow tsconfig\.base\.json
```
