---
reasonCode: tsconfig-not-solution-file
nextAction: Make tsconfig.json a solution file, `"files": []` with `"references"` and no `include`, and move the compiled files into the role configs it references.
---

# `tsconfig.json` is a solution file

A `tsconfig.json` must declare `"files": []` and must not declare `include`. It
owns no source file; it only lists, in `references`, the role configs that do.

A `tsconfig.json` that compiles files itself is a second, unnamed role beside
the role configs. `tsc` and `tsc -b` then check those files under whatever
`lib` and `types` it carries, and a file matched by both it and a role config is
checked twice under two environments. A solution file keeps each source file in
exactly one role.

Fix it by moving `include` to the role config that owns those files, and setting
`"files": []`.

```grit
language json
multifile {
  file($name, $body) where {
    $name <: r"(?:.*/)?tsconfig\.json",
    $program <: or {
      not contains `"files": []`,
      contains `"include": $_`
    }
  }
}
```
