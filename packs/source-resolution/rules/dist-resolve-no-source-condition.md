---
reasonCode: dist-resolve-exports-condition
nextAction: Remove the source condition from this package's export map (and `devExports` from its bundler config), or delete its `"gritlint": { "resolve": "dist" }` declaration.
locate: `$cond: $_` as $pair where { $cond <: condition(), $pair <: within `"exports": $_` as $map where { $map <: not within `"publishConfig": $_` } }
---

# A package declared to resolve to `dist/` carries no source condition

A `package.json` that declares `"gritlint": { "resolve": "dist" }` says its
workspace consumers read its built `dist/`, not its `src/`. Its export map must
therefore carry the source condition on no subpath: the condition would send
every consumer that names it back to `src/`, against the declaration.

A bundler config that still sets `devExports` writes the condition back into the
export map on its next build, and this rule then fails on the rewritten map.

`publishConfig.exports` is left to `publish-config-exports`.

Fix it by removing the condition from the export map and `devExports` from the
bundler config, or by deleting the declaration so the package resolves to source
again.

```grit
language json
multifile {
  file($name, $body) where {
    $name <: r"(?:.*/)?package\.json",
    $program <: contains `"gritlint": { "resolve": "dist" }` as $decl where {
      $decl <: not within `$key: $_` where { $key <: not `"gritlint"` }
    },
    $program <: contains `"exports": $exports` as $map where {
      $map <: not within `"publishConfig": $_`,
      $exports <: contains `$cond: $_` where {
        $cond <: condition()
      }
    }
  }
}
```
