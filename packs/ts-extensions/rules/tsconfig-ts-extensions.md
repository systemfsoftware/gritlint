---
reasonCode: tsconfig-allows-ts-extensions
nextAction: Remove `allowImportingTsExtensions` and `rewriteRelativeImportExtensions` from `compilerOptions`, then import with extensionless or `.js` specifiers.
locate: or { `"allowImportingTsExtensions": true`, `"rewriteRelativeImportExtensions": true` } as $option where { $option <: within `"compilerOptions": $_` }
---

# tsconfig: no option that lets a specifier name a TypeScript file

`compilerOptions` must not set `allowImportingTsExtensions` or
`rewriteRelativeImportExtensions` to `true`.

Both options exist so that a module specifier can end in `.ts`, `.tsx`, `.mts`
or `.cts`. A specifier that names the source file ties the import to the file
that is on disk during development, not to the module a consumer resolves. The
emitted `.js` and `.d.ts` then keep the `.ts` specifier (the first option only
type-checks with `noEmit` or `emitDeclarationOnly`) or depend on the compiler
rewriting it (the second), and a bundler, a runtime or a consumer that resolves
specifiers on its own breaks on a path that does not exist in the published
output.

Fix it by deleting the option and writing every specifier without an extension,
or with the `.js` extension that the emitted file will carry. With both options
off, the compiler rejects such a specifier in an `import`, `export ... from` or
`import()` of a file it checks (TS5097), though not in a type-position
`import()` type.

```grit
language json

or {
  `"allowImportingTsExtensions": true`,
  `"rewriteRelativeImportExtensions": true`
} as $option where {
  $option <: within `"compilerOptions": $_`
}
```
