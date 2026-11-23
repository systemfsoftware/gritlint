---
reasonCode: import-specifier-build-output
nextAction: Import the package by its name and a subpath its `exports` map declares, instead of a path that contains a `dist` segment.
locate: pattern build_output_specifier() { r"['\"`](?:(?:\./)*(?:\.\./)+[^./'\"`][^/'\"`]*/(?:[^/'\"`]+/)*dist|/(?:[^/'\"`]+/)*dist|@[^/'\"`]+/[^/'\"`]+/(?:[^/'\"`]+/)*dist|[^@.#/'\"`][^/'\"`]*/(?:[^/'\"`]+/)*dist)(?:/[^'\"`]*)?['\"`]" } or { import_statement(source=$source) where { $source <: build_output_specifier() }, export_statement(source=$source) where { $source <: build_output_specifier() }, call_expression(function=$callee, arguments=$arguments) where { $callee <: r"import", $arguments <: [$first, $...] where { $first <: build_output_specifier() } }, import_require_clause(source=$source) where { $source <: build_output_specifier() } }
---

# An import specifier does not reach into a package's build output

A module specifier in import position must not resolve by path into another
package's `dist/` directory. That covers a static `import`, an `export ... from`,
a dynamic `import()` and an `import x = require()`, type-only forms included.
Three shapes fail:

- a relative specifier that climbs out with `../`, names another directory, and
  then reaches a `dist` segment (`../other/dist/index.js`,
  `../../packages/x/dist/y.mjs`);
- an absolute specifier with a `dist` segment;
- a package specifier whose subpath has a `dist` segment (`pkg/dist/y.js`,
  `@scope/pkg/dist/y.js`).

A path into build output skips the package's `exports` map, so it ties the
importer to the build layout, to a build that must have run first, and to files
the package never promised. The package name plus a subpath its `exports` map
declares is the contract, and it resolves the same in the workspace, in the
published tarball and under a bundler.

Not judged: a relative path that stays in the importing package (`./dist/x.js`)
or climbs straight to its own `dist` (`../dist/x.js`, `../../dist/x.js`), because
a specifier alone cannot show which package owns the directory; and any string
that is not a module specifier (a bundler `outDir`, a `new URL()` argument, a
`vi.mock()` call, a comment).

Fix it by importing `pkg` or `pkg/<subpath>` as the package's `exports` declares.

```grit
language js

or {
  import_statement(source=$source) where { $source <: build_output_specifier() },
  export_statement(source=$source) where { $source <: build_output_specifier() },
  call_expression(function=$callee, arguments=$arguments) where {
    $callee <: r"import",
    $arguments <: [$first, $...] where { $first <: build_output_specifier() }
  },
  import_require_clause(source=$source) where { $source <: build_output_specifier() }
}

pattern build_output_specifier() {
  r"['\"`](?:(?:\./)*(?:\.\./)+[^./'\"`][^/'\"`]*/(?:[^/'\"`]+/)*dist|/(?:[^/'\"`]+/)*dist|@[^/'\"`]+/[^/'\"`]+/(?:[^/'\"`]+/)*dist|[^@.#/'\"`][^/'\"`]*/(?:[^/'\"`]+/)*dist)(?:/[^'\"`]*)?['\"`]"
}
```
