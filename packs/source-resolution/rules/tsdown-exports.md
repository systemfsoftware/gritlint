---
reasonCode: tsdown-exports-misconfigured
nextAction: Declare `exports.customExports`, set `devExports` to the condition string or leave it out, and delete any `clean: false`.
locate: or { `clean: false`, `devExports: $value` where { $value <: not condition() }, `exports: $exports` where { $exports <: `{ $first, $... }`, $exports <: not contains `customExports: $_` } }
---

# tsdown config: the hook, a named source condition when there is one, and cleaning on

The bundler config must own both export maps: `exports.customExports` names the
hook that injects `types` on every subpath, `exports.devExports`, when present,
names the development source condition as a **string**, and `clean` must not be
disabled. A config may leave `devExports` out: whether its package must expose
the condition is decided on `package.json` by `tsdown-source-condition-required`
and `dist-resolve-no-source-condition`, which can read the declaration this
JavaScript rule cannot.

`devExports: true` points the whole map at source with no condition key, so
`npm pack` would ship a map whose `default` resolves to a file the tarball does
not contain, and a consumer that does not read the condition gets source instead
of declarations. A missing `customExports` hook leaves the generated map without
`types`, which is the untyped-resolution finding. `clean: false` leaves build
output in `dist/` that no current entry emits — orphaned output that every gate
globbing the directory reads, so a renamed entry or dropped subpath keeps
resolving from a stale declaration file.

Fix it by declaring `exports: { devExports: '<scope>/source', customExports:
injectTypes }` (or `customExports` alone for a package that resolves to `dist/`), with `injectTypes` visiting every export key except
`./package.json` on both hook invocations, and by deleting any `clean: false`.

```grit
language js
multifile {
  file($name, $body) where {
    $name <: r"(?:.*/)?tsdown\.config\.ts",
    $program <: or {
      contains `exports: $exports` where {
        $exports <: `{ $first, $... }`,
        $exports <: or {
          not contains `customExports: $_`,
          contains `devExports: $value` where {
            $value <: not condition()
          }
        }
      },
      contains `clean: false`
    }
  }
}
```
