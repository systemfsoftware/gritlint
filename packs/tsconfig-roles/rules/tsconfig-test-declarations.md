---
reasonCode: tsconfig-test-declarations
nextAction: Set `composite`, `declaration` and `emitDeclarationOnly` to false in the test config, or remove them.
locate: or { `"compilerOptions": { $..., "composite": true, $... }`, `"compilerOptions": { $..., "declaration": true, $... }`, `"compilerOptions": { $..., "emitDeclarationOnly": true, $... }`, `"extends": $_` } as $site where { $filename <: r"(?:.*/)?tsconfig\.test\.json" }
---

# `tsconfig.test.json` computes no declarations

A test project must not set `composite`, `declaration` or `emitDeclarationOnly`
to `true`, directly or through a same-directory file it `extends`, and must set
all three to `false` when it extends a file gritlint cannot read.

With any of the three on, the test project computes a `.d.ts` for every module
it compiles. `composite` does so even under `noEmit`, because it turns on
declaration diagnostics. Test projects compile stories, fixtures and helpers,
and an exported helper whose inferred type contains a large non-exported type —
a router's route tree, for example — makes the declaration printer inline that
type at every use site. The compiler can then exhaust memory before it reports
TS4023 or TS7056. Nothing consumes a test project's declarations, so the cost
buys nothing.

A non-composite test project can stay in its solution's `references`: `tsc -b`
builds it, skips it when it is up to date, and reports its errors. That is why
`tsconfig-test-unreferenced` still requires the reference.

Fix it by setting all three to `false` in `tsconfig.test.json`, or by removing
them when the file it `extends` already leaves declarations off.

A flag set by another role's config is not the test project's: only the file the
test config `extends` counts, so a `tsconfig.app.json` that computes
declarations does not fail `tsconfig.test.json`.

The rule follows an `extends` entry written `./<file>` (or `.\\<file>`), a
string or an item of the array form, naming a file in the same directory, one
hop. A `false` the test config sets for a flag overrides that flag from the file
it extends, so an inherited `true` is judged only when the test config does not
turn that same flag off. It does not follow a preset reached through a package
name (`@scope/tsconfig/test.json`), an `extends` that climbs to another
directory, or a chain of more than one `extends`. A config reached through any
of them must carry the three explicit `false` values itself, because gritlint
cannot read what it sets; the rule requires them for every `extends` entry it
cannot follow.

A file with comments is parsed and judged. A trailing comma does not parse, so
gritlint reports `target-does-not-parse` for the file and refuses the run; it
never skips a file silently.

```grit
language json
multifile {
  bubble($dir, $base, $key) file($name, $body) where {
    $name <: r"(.*/|)([^/]+)\.json"($base_dir, $base),
    $base_dir <: $dir,
    $program <: contains `{ $... }` as $root where { $root <: not within or { pair(), array() } },
    or {
      $root <: `{ $..., "compilerOptions": { $..., $key: true, $... }, $... }` where {
        $key <: or { `"composite"`, `"declaration"`, `"emitDeclarationOnly"` }
      },
      and {
        $name <: r"(?:.*/)?tsconfig\.test\.json",
        $key = $name
      }
    }
  },
  bubble($dir, $base, $key) file($name, $body) where {
    $name <: r"(.*/|)tsconfig\.test\.json"($test_dir),
    $test_dir <: $dir,
    $program <: contains `{ $... }` as $root where { $root <: not within or { pair(), array() } },
    $root <: or {
      `{ $..., "compilerOptions": { $..., "composite": true, $... }, $... }`,
      `{ $..., "compilerOptions": { $..., "declaration": true, $... }, $... }`,
      `{ $..., "compilerOptions": { $..., "emitDeclarationOnly": true, $... }, $... }`,
      `{ $..., "extends": $ext, $... }` where {
        $ext <: contains r"\"(?:\./|\.\\\\)(.+?)(?:\.json)?\""($extended) where {
          $extended <: $base
        },
        $root <: not `{ $..., "compilerOptions": { $..., $key: false, $... }, $... }`
      },
      `{ $..., "extends": $ext, $... }` where {
        $ext <: contains r"\"(?:\.\.|\./[^\"]*(?:/|\\\\)|\.\\\\[^\"]*(?:/|\\\\)|/|\\\\|[A-Za-z]:|[^.\"/\\\\])[^\"]*\"",
        $root <: not `{ $..., "compilerOptions": $options, $... }` where {
          $options <: contains `"composite": false`,
          $options <: contains `"declaration": false`,
          $options <: contains `"emitDeclarationOnly": false`
        }
      }
    }
  }
}
```
