---
reasonCode: import-specifier-ts-extension
nextAction: Write the specifier without an extension, or with the `.js` extension the emitted file carries, instead of `.ts`, `.tsx`, `.mts` or `.cts`.
locate: pattern ts_specifier() { r"['\"`](?:\.{1,2}/|/|#|@[^/'\"`]+/[^/'\"`]+/|[^@#/'\"`][^/'\"`]*/)[^'\"`]*\.(?:ts|tsx|mts|cts)(?:[?#][^'\"`]*)?['\"`]" } or { import_statement(source=$source) where { $source <: ts_specifier() }, export_statement(source=$source) where { $source <: ts_specifier() }, call_expression(function=$callee, arguments=$arguments) where { $callee <: r"import", $arguments <: [$first, $...] where { $first <: ts_specifier() } }, import_require_clause(source=$source) where { $source <: ts_specifier() } }
---

# An import specifier names no TypeScript file

A module specifier in import position must not end in `.ts`, `.tsx`, `.mts` or
`.cts`. That covers a static `import`, an `export ... from`, a dynamic
`import()`, an `import x = require()`, and an `import()` type, and it covers
type-only forms too. It applies to every subpath specifier: a relative or
absolute path, a `#` subpath import, and a package subpath such as
`pkg/dist/x.ts`. A bare package name that happens to end in `.ts` has no subpath
and is not a specifier of this kind.

A `.ts` specifier names the source file that is on disk during development, not
the module a consumer resolves. It needs `allowImportingTsExtensions` or
`rewriteRelativeImportExtensions` to type-check, the emitted `.js` and `.d.ts`
keep a path to a file the published package does not contain, and a bundler or
runtime that resolves specifiers itself fails on it. A `.js` specifier names the
file the build emits and a specifier without an extension names the module, so
both resolve the same before and after the build.

A bundler query or fragment suffix (`./y.ts?raw`) does not hide the extension.

Strings that are not module specifiers are not judged: a path in a bundler or
test-runner config, a `new URL()` argument, a `vi.mock()` or `require()` call
argument, or any other string literal that ends in `.ts`.

Fix it by dropping the extension, or by writing `.js`. The sibling rule
`tsconfig-ts-extensions` fails the compiler options that allow the extension.

```grit
language js

or {
  import_statement(source=$source) where { $source <: ts_specifier() },
  export_statement(source=$source) where { $source <: ts_specifier() },
  call_expression(function=$callee, arguments=$arguments) where {
    $callee <: r"import",
    $arguments <: [$first, $...] where { $first <: ts_specifier() }
  },
  import_require_clause(source=$source) where { $source <: ts_specifier() }
}

pattern ts_specifier() {
  r"['\"`](?:\.{1,2}/|/|#|@[^/'\"`]+/[^/'\"`]+/|[^@#/'\"`][^/'\"`]*/)[^'\"`]*\.(?:ts|tsx|mts|cts)(?:[?#][^'\"`]*)?['\"`]"
}
```
