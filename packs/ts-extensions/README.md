# ts-extensions

Two rules that keep TypeScript file extensions out of module specifiers:

- `tsconfig-ts-extensions` fails a `compilerOptions` object that sets
  `allowImportingTsExtensions` or `rewriteRelativeImportExtensions` to `true`.
- `ts-extension-specifier` fails a module specifier that ends in `.ts`, `.tsx`,
  `.mts` or `.cts` (optionally followed by a `?query`) in a static `import`, an
  `export ... from`, a dynamic `import()` or an `import x = require()`, type-only
  forms included. It judges relative, absolute, `#` subpath and package subpath
  specifiers. A bare package name that ends in `.ts` has no subpath and passes.

A specifier that names the source file ties an import to the file on disk during
development instead of the module a consumer resolves. Write the specifier
without an extension, or with the `.js` extension the build emits.

The rules read syntax, never a name or a position. A string that ends in `.ts`
but is not a module specifier (a bundler or test-runner entry path, a
`new URL()` argument, a `vi.mock()` or `require()` argument, a comment) is not
judged.

Each module specifier is one finding. A file the grammar cannot parse is
reported as `target-does-not-parse` and never skipped silently.

## Enable it

```json
{
  "packs": {
    "ts-extensions": {}
  }
}
```

No parameters.
