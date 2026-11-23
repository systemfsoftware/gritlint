# @systemfsoftware/gritlint@0.6.0

`tsconfig-role-unreferenced` is split into `tsconfig-app-unreferenced`, `tsconfig-node-unreferenced` and `tsconfig-test-unreferenced`, and `tsconfig.build.json` is no longer judged.

- A finding now points at the unreferenced role file. The single rule reported the first file of the directory that took part in the join, which was often a role file the solution does reference.
- A build config is an emit-only, non-composite project, and `tsc -b` refuses to reference it (TS6306), so requiring a reference to it failed every package that follows that layout.
