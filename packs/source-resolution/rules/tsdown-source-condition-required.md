---
reasonCode: tsdown-exports-condition-missing
nextAction: Set `exports.devExports` to the condition string in the bundler config and rebuild, or declare `"gritlint": { "resolve": "dist" }` in this package.json.
locate: or { `"exports": $value` as $map where { $map <: not within `"publishConfig": $_`, $value <: or { r"\"\./dist/.*\"", and { not contains `$key: $_` where { $key <: r"\"\..*\"" }, contains `$_: $target` where { $target <: r"\"\./dist/.*\"" }, not contains `$cond: $_` where { $cond <: condition() } } } }, `$subpath: $entry` as $pair where { $subpath <: r"\"\..*\"", $pair <: within `"exports": $_` as $map where { $map <: not within `"publishConfig": $_` }, $entry <: or { r"\"\./dist/.*\"", and { contains `$_: $target` where { $target <: r"\"\./dist/.*\"" }, not contains `$cond: $_` where { $cond <: condition() } } } } }
---

# A bundled package that is not declared `dist` exposes the source condition

A package with a bundler config (`tsdown.config.ts`) resolves to its own `src/`
for workspace consumers unless its `package.json` declares
`"gritlint": { "resolve": "dist" }`. Every export entry that points at built
output — a string into `./dist/`, or an object with any target into `./dist/` —
must then be an object that carries the source condition, and so must a root
conditions map with no subpaths. A stylesheet or other asset entry that points
only at `src/` is not built output and needs none. A package with any top-level
`"gritlint"` key is left to `dist-resolve-no-source-condition` and
`gritlint-resolve-value`. `publishConfig.exports` is the
registry's map, which carries no condition (`publish-config-exports`), so it is
not checked here.

Without the condition such a subpath sends consumers to `dist/`, which a clean
checkout does not have, without the declaration that says so. This is the rule
that keeps `devExports` optional in `tsdown-exports`: a config that leaves it
out must belong to a declared package.

Fix it by setting `exports.devExports` to the condition string and rebuilding,
or by declaring `"gritlint": { "resolve": "dist" }`.

```grit
language json
multifile {
  bubble($dir) file($name, $body) where {
    $name <: r"(.*/|)tsdown\.config\.ts"($dir)
  },
  bubble($dir) file($name, $body) where {
    $name <: r"(.*/|)package\.json"($package_dir),
    $package_dir <: $dir,
    $program <: not contains `"gritlint": $_` as $decl where {
      $decl <: not within `$key: $_` where { $key <: not `"gritlint"` }
    },
    $program <: contains `"exports": $exports` as $map where {
      $map <: not within `"publishConfig": $_`,
      $exports <: or {
        r"\"\./dist/.*\"",
        and {
          not contains `$key: $_` where { $key <: r"\"\..*\"" },
          contains `$_: $target` where { $target <: r"\"\./dist/.*\"" },
          not contains `$cond: $_` where { $cond <: condition() }
        },
        contains `$subpath: $entry` where {
          $subpath <: r"\"\..*\"",
          $entry <: or {
            r"\"\./dist/.*\"",
            and {
              contains `$_: $target` where { $target <: r"\"\./dist/.*\"" },
              not contains `$cond: $_` where { $cond <: condition() }
            }
          }
        }
      }
    }
  }
}
```
