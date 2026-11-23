---
reasonCode: tsconfig-dom-extends-runtime-types
nextAction: Split the config by runtime: keep `DOM` in the browser role (tsconfig.app.json) and `node` or `@cloudflare/workers-types` in the runtime role (tsconfig.node.json), and extend a base that sets neither.
---

# A config with `DOM` does not inherit `node` or Worker types

A `tsconfig*.json` that sets `lib` with `DOM` and no `types` of its own, and
`extends` a file in the same directory whose `compilerOptions.types` lists `node`,
`@types/node` or `@cloudflare/workers-types`, resolves both: the browser and the
server globals load together. `types` is inherited when the child does not set
it, so the child's `DOM` and the parent's runtime types meet in one project.

Only an `extends` entry written `./<file>` (or `.\\<file>`), a string or an item
of the array form, naming a file in the same directory is followed, one hop. gritlint evaluates a multifile rule one directory at a time (the
engine groups the scanned files by parent directory), so an `extends` that
climbs (`../../tsconfig.base.json`) names a file in another directory that the
rule never sees. A preset reached by package name (`@scope/tsconfig/app.json`)
is a path under `node_modules` or a workspace package and is not a file of this
directory either. Neither is followed, and a combination that exists only
through them is not seen. A child's own `lib` or `types` replaces the parent's
(`extends` does not merge arrays), so a child that sets the key itself is not
judged against the parent's value.

```grit
language json
multifile {
  bubble($dir, $ext) file($name, $body) where {
    $name <: r"(.*/|)tsconfig(?:\.[^/]+)?\.json"($dir),
    $program <: contains `{ $... }` as $root where {
      $root <: not within or { pair(), array() },
      $root <: `{ $..., "extends": $ext, $... }`
    },
    $ext <: contains r"\"(?:\./|\.\\\\).+\"",
    $program <: contains `"compilerOptions": { $... }` as $options where {
      $options <: `"compilerOptions": { $..., "lib": $lib, $... }` where { $lib <: contains r"\"(?i:dom(?:\.[a-z]+)?|lib\.dom(?:\.[a-z]+)?\.d\.ts)\"" },
      $options <: not `"compilerOptions": { $..., "types": $_, $... }`
    }
  },
  bubble($dir, $ext) file($name, $body) where {
    $name <: r"(.*/|)([^/]+)\.json"($parent_dir, $parent_base),
    $parent_dir <: $dir,
    $ext <: contains r"\"(?:\./|\.\\\\)(.+?)(?:\.json)?\""($extended) where { $extended <: $parent_base },
    $program <: contains `"compilerOptions": { $..., "types": $value, $... }` where {
      $value <: contains r"\"(?:node|@types/node|@cloudflare/workers-types(?:/[^\"]*)?)\""
    }
  }
}
```
