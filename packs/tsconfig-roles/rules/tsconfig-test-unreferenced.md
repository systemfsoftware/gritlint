---
reasonCode: tsconfig-test-unreferenced
nextAction: Add `{ "path": "./tsconfig.test.json" }` to the `references` of the `tsconfig.json` beside it, or delete tsconfig.test.json.
locate: or { `"compilerOptions": $_`, `"extends": $_`, `"include": $_` }
---

# `tsconfig.test.json` is referenced by the `tsconfig.json` beside it

A `tsconfig.test.json` in a directory that has a `tsconfig.json` must be one of
that file's `references`, written `./tsconfig.test.json` (or `tsconfig.test.json`, or
with a `.\\` prefix).

`tsc -b` starts at the solution file and builds only what it references. A role
file it does not reference is never typechecked by `tsc -b`, so the files it
owns report clean on a tree that does not compile.

Fix it by adding `{ "path": "./tsconfig.test.json" }` to the solution file's
`references`.

Each role has its own rule so a finding names the role file that is missing: the
finding points at the first `compilerOptions`, `extends` or `include` of the
files in the directory that took part in the join, which is that role file. A
role file with none of those keys is reported at the `tsconfig.json` of the
directory.

`tsconfig.build.json` has no such rule: a build config is an emit-only project
(`composite: false`), and `tsc -b` refuses to reference a project that is not
composite (TS6306), so referencing it would break the solution build.

A directory with role files and no `tsconfig.json` at all is not judged, and
neither is a reference whose target file is missing: gritlint joins files by
presence and cannot assert that a file is absent. `tsc -b` itself reports a
reference to a missing project (TS6053).

```grit
language json
multifile {
  bubble($dir, $references) file($name, $body) where {
    $name <: r"(.*/|)tsconfig\.json"($dir),
    $program <: contains `{ $... }` as $root where {
      $root <: not within or { pair(), array() },
      $root <: or {
        `{ $..., "references": $references, $... }`,
        not `{ $..., "references": $_, $... }` where { $references = `[]` }
      }
    }
  },
  bubble($dir, $references) file($name, $body) where {
    $name <: r"(.*/|)tsconfig\.test\.json"($role_dir),
    $role_dir <: $dir,
    $references <: not contains `"path": $path` where {
      $path <: r"\"(?:\./|\.\\\\)?tsconfig\.test\.json\""
    }
  }
}
```
