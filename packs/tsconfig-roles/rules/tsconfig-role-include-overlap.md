---
reasonCode: tsconfig-role-include-overlap
nextAction: Give each role config its own `include` entries, or exclude the other role's files, so no file is checked under two roles.
---

# Two role configs in one directory do not include the same entry

`tsconfig.app.json`, `tsconfig.node.json` and `tsconfig.test.json` in one
directory must not list the same `include` entry (the same string) when neither
declares `exclude`. Both keys are read at the top level of the file.

A file matched by two role configs is checked under two sets of `lib` and
`types`, so it can pass one and break the other, and which result a tool shows
depends on which project it opens the file in.

Only the provable case is judged: an identical `include` string in two role
configs that have no `exclude`. Different strings that match overlapping files
(`src` and `src/**/*.ts`), and an `include` inherited through `extends`, are not
compared: gritlint compares strings and does not expand globs or merge `extends`.
A pair where either file declares `exclude` is not judged, because the exclusion
can make the sets disjoint.

```grit
language json
multifile {
  bubble($dir, $first, $entries) file($name, $body) where {
    $first = $name,
    $name <: r"(.*/|)tsconfig\.(?:app|node|test)\.json"($dir),
    $program <: contains `{ $... }` as $root where {
      $root <: not within or { pair(), array() },
      $root <: not `{ $..., "exclude": $_, $... }`,
      $root <: `{ $..., "include": $entries, $... }`
    }
  },
  bubble($dir, $first, $entries) file($name, $body) where {
    $name <: r"(.*/|)tsconfig\.(?:app|node|test)\.json"($other_dir),
    $other_dir <: $dir,
    $name <: not $first,
    $program <: contains `{ $... }` as $root where {
      $root <: not within or { pair(), array() },
      $root <: not `{ $..., "exclude": $_, $... }`,
      $root <: `{ $..., "include": $own, $... }`,
      $own <: contains r"\".*\"" as $entry where { $entries <: contains $entry }
    }
  }
}
```
