# tsconfig-roles

Eleven rules that keep a repository's tsconfig files to a closed set of roles. The
last five are multifile rules, written like `typecheck-build-mode`: they join the
files of one directory.

- `tsconfig-file-name` (path rule) fails a `tsconfig*.json` that is not
  `tsconfig.json`, `tsconfig.app.json`, `tsconfig.node.json`, `tsconfig.test.json`
  or `tsconfig.build.json`. `tsconfig.base.json` is allowed only at the repository
  root. It judges the file's path, so a name invented for a feature or a folder
  (`tsconfig.page.json`) fails whatever it contains.
- `tsconfig-solution-file` fails a `tsconfig.json` that does not declare a
  top-level `"files": []` or that declares `include`.
- `tsconfig-reference-outside-package` fails a `tsconfig.json` `references` path
  that has a `..` segment or is absolute (`/x`, `C:/x`, `\\x`).
- `tsconfig-dom-with-runtime-types` fails one `tsconfig*.json` whose own
  `compilerOptions` lists `DOM` (or `DOM.Iterable`, `DOM.AsyncIterable`,
  `lib.dom.d.ts`) in `lib` and `node`, `@types/node` or
  `@cloudflare/workers-types` in `types`.
- `tsconfig-app-unreferenced`, `tsconfig-node-unreferenced` and `tsconfig-test-unreferenced` each fail a role file
  that the `tsconfig.json` in the same directory does not reference as
  `./tsconfig.<role>.json`. A finding points at the role file (at its first
  `compilerOptions`, `extends` or `include`). `tsconfig.build.json` is not judged: an
  emit-only, non-composite project cannot be referenced by `tsc -b` (TS6306).
- `tsconfig-role-include-overlap` fails two of `app`, `node` and `test` in one
  directory that list the same `include` string when neither declares `exclude`.
- `tsconfig-dom-extends-runtime-types` fails a config with `DOM` in `lib` and no
  `types` of its own that `extends` a same-directory file whose `types` lists a
  runtime (`node`, `@types/node`, `@cloudflare/workers-types`).
- `tsconfig-runtime-types-extend-dom` fails the mirror: runtime `types`, no `lib`
  of its own, `extends` a same-directory file whose `lib` lists `DOM`.
- `tsconfig-test-declarations` fails a `tsconfig.test.json` that turns on
  `composite`, `declaration` or `emitDeclarationOnly`, in its own
  `compilerOptions` or through a same-directory `extends`, and one that extends a
  file gritlint cannot follow without setting all three to `false` itself. A test
  project's declarations have no consumer, and computing them can inline a large
  non-exported type at every use site.

## What the rules cannot see

Each finding is evidence of a failure, and each rule below states what it does not
cover.

- **One directory at a time.** gritlint evaluates a multifile rule on the files
  of one directory (`partition_multifile` in the engine groups the scanned files by
  parent directory). A multifile join across directories, such as a child config
  and the `../../tsconfig.base.json` it extends, never sees the second file. The
  `extends` rules therefore follow only `"extends": "./<file>"` in the same
  directory, one hop.
- **`extends` through a package name** (`@scope/tsconfig/app.json`) points into
  `node_modules` or a workspace package, not into the directory. It is not
  followed. A combination of `DOM` and runtime types that exists only through
  such a preset, or through an ancestor-directory base, is not seen. A repository
  that keeps its shared base at the root and extends it as `../../tsconfig.base.json`
  gets the single-file `tsconfig-dom-with-runtime-types` only.
- **`references` that name a missing file.** gritlint joins files that exist and
  cannot assert that a file is absent: a negated file clause in a multifile rule
  does not test for non-existence. `tsc -b` reports a reference to a missing
  project itself (TS6053). A directory with role files and no `tsconfig.json` is
  likewise not judged.
- **Directory references.** The three `unreferenced` rules read references written
  `./tsconfig.<role>.json` (or without `./`). A reference to a directory
  (`./sub`) is not a role reference.
- **Include overlap is compared as strings.** `tsconfig-role-include-overlap`
  flags an identical `include` string in two role configs that have no `exclude`.
  It does not expand globs: `src` and `src/**/*.ts` overlap and are not flagged,
  an `include` inherited through `extends` is not compared, and a pair where
  either file declares `exclude` is not judged. Whether a `tsconfig.app.json` or
  `tsconfig.build.json` matches test files, and the resolved `noEmit` or
  `declaration` of each role, need glob expansion and a merged `extends` and are
  not checked.
- A multifile finding is one per directory and points at a file of that directory,
  not always the offending role file.
- A tsconfig with a trailing comma is reported as `target-does-not-parse`, never
  skipped silently.

## Enable it

```json
{
  "packs": {
    "tsconfig-roles": {}
  }
}
```

No parameters. A tree that keeps fixtures or vendored tsconfig files names them in
the config's `ignore` globs.
