---
reasonCode: tsconfig-reference-leaves-package
nextAction: Reference only role configs in the same directory (`./tsconfig.app.json`); depend on another package through its package name, not a tsconfig reference.
---

# `tsconfig.json` references stay inside its own directory

A `tsconfig.json` `references` entry names a path inside the directory of the
file that declares it. A path with a `..` segment, or an absolute path, reaches
another package's project.

A reference across a package boundary makes this package's `tsc -b` build the
other package's project graph as well, so one package's typecheck pulls in and
can fail on another's, and the build graph becomes a single large project
instead of one small graph per package. A package that uses another one depends
on it by name, through its `package.json`.

Fix it by pointing `references` at the role configs beside the file and removing
the cross-package entry.

```grit
language json
multifile {
  file($name, $body) where {
    $name <: r"(?:.*/)?tsconfig\.json",
    $program <: contains `"references": $references` where {
      $references <: contains `"path": $path` where {
        $path <: r"\"(?:/.*|(?:.*/)?\.\.(?:/.*)?)\""
      }
    }
  }
}
```
