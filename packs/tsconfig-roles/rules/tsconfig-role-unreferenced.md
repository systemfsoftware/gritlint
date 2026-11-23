---
reasonCode: tsconfig-role-unreferenced
nextAction: Add a `references` entry for this role file to the `tsconfig.json` beside it, or delete the role file.
---

# Every role file is referenced by the `tsconfig.json` beside it

A `tsconfig.app.json`, `tsconfig.node.json`, `tsconfig.test.json` or
`tsconfig.build.json` in a directory that has a `tsconfig.json` must be one of
that file's `references`, written `./tsconfig.<role>.json`.

`tsc -b` starts at the solution file and builds only what it references. A role
file it does not reference is never typechecked by `tsc -b`, so the files it
owns report clean on a tree that does not compile.

Fix it by adding `{ "path": "./tsconfig.<role>.json" }` to the solution file's
`references`.

A directory with role files and no `tsconfig.json` at all is not judged, and
neither is a reference whose target file is missing: gritlint joins files by
presence and cannot assert that a file is absent. `tsc -b` itself reports a
reference to a missing project (TS6053).

```grit
language json
multifile {
  bubble($dir, $references) file($name, $body) where {
    $name <: r"(.*/|)tsconfig\.json"($dir),
    $program <: or {
      contains `"references": $references`,
      not contains `"references": $_` where { $references = `[]` }
    }
  },
  bubble($dir, $references) file($name, $body) where {
    $name <: r"(.*/|)tsconfig\.(app|node|test|build)\.json"($role_dir, $role),
    $role_dir <: $dir,
    $references <: not contains `"path": $path` where {
      $path <: r"\"(?:\./)?tsconfig\.(app|node|test|build)\.json\""($referenced),
      $referenced <: $role
    }
  }
}
```
