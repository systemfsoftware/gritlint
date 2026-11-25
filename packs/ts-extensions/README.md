# ts-extensions

One rule that keeps TypeScript file extensions out of module specifiers:
`tsconfig-ts-extensions` fails a `compilerOptions` object that sets
`allowImportingTsExtensions` or `rewriteRelativeImportExtensions` to `true`.

Without either option the TypeScript compiler itself rejects a specifier that
ends in `.ts`, `.tsx`, `.mts` or `.cts` (TS5097), so the pack reads only the
tsconfig files and never scans source. Write the specifier without an
extension, or with the `.js` extension the build emits.

## Enable it

```json
{
  "packs": {
    "ts-extensions": {}
  }
}
```

No parameters.
