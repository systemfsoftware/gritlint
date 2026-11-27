---
reasonCode: gritlint-declaration-invalid
nextAction: Write the declaration as exactly `"gritlint": { "resolve": "dist" }`, or delete it to resolve to source.
locate: `"gritlint": $value` as $decl where { $decl <: not within `$key: $_` where { $key <: not `"gritlint"` }, $value <: not `{ "resolve": "dist" }` }
---

# `"gritlint"` in `package.json` is exactly `{ "resolve": "dist" }`

`"gritlint": { "resolve": "dist" }` is the only package-level declaration: it
moves a package's workspace consumers from its `src/` to its `dist/`. Leaving the
key out is the other mode. Any other value — another `resolve` value, a
misspelled key, an extra key, or a value that is not an object — would read as a
declaration to its author while every rule treats the package as undeclared.

Only the top-level key counts: a `"gritlint"` key nested in another object, such
as a `bin` entry, is not a declaration.

Fix it by writing exactly `"gritlint": { "resolve": "dist" }`, or by deleting the
key.

```grit
language json
multifile {
  file($name, $body) where {
    $name <: r"(?:.*/)?package\.json",
    $program <: contains `"gritlint": $value` as $decl where {
      $decl <: not within `$key: $_` where { $key <: not `"gritlint"` },
      $value <: not `{ "resolve": "dist" }`
    }
  }
}
```
