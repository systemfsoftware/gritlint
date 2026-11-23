# tsconfig-roles

Four rules that keep a repository's tsconfig files to a closed set of roles.

- `tsconfig-file-name` fails a `tsconfig*.json` that is not `tsconfig.json`,
  `tsconfig.app.json`, `tsconfig.node.json`, `tsconfig.test.json` or
  `tsconfig.build.json`. `tsconfig.base.json` is allowed only at the repository
  root. It judges the file's path, so a name invented for a feature or a folder
  (`tsconfig.page.json`) fails whatever it contains.
- `tsconfig-solution-file` fails a `tsconfig.json` that does not declare
  `"files": []` or that declares `include`: the file owns no source, the role
  configs it references do.
- `tsconfig-reference-outside-package` fails a `tsconfig.json` `references` path
  that has a `..` segment or is absolute, because it reaches another package's
  project.
- `tsconfig-dom-with-runtime-types` fails one `compilerOptions` that lists `DOM`
  (or `DOM.Iterable`, `DOM.AsyncIterable`, `lib.dom.d.ts`) in `lib` and `node`,
  `@types/node` or `@cloudflare/workers-types` in `types`.

## What the rules cannot see

Each rule reads one file at a time and never resolves `extends`.

- A `lib` or `types` value that reaches a config only through `extends` is not
  seen by `tsconfig-dom-with-runtime-types`.
- Whether a `tsconfig.json`'s `references` equal exactly the role files beside it
  is not checked: it needs the directory listing, which a content rule does not
  have.
- Whether the role configs' file sets overlap, and what each role resolves for
  `noEmit` or `declaration`, need `include` globs expanded and `extends` merged.
  Neither is checked.
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
