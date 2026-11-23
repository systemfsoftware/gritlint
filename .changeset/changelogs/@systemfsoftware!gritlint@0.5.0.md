# @systemfsoftware/gritlint@0.5.0

An `import()` type now parses with a qualified member name (`import('react').JSX.Element`) and with an attributes object (`import('x', { with: { 'resolution-mode': 'import' } }).T`), and `ts-extension-specifier` and `dist-import-specifier` read only its module string, not a string in its attributes or type arguments.

gritlint parses four TypeScript constructs it refused before, and `gritlint check` reports every file it cannot scan in one run.

- A type parameter with a variance annotation (`<out T, in U, in out W>`), an `import('x').T` type, `export type * from 'x'` and `export type * as ns from 'x'` now parse. `import('x').T` is a new `import_type` node, which `ts-extension-specifier` and `dist-import-specifier` judge.
- A JSX text or attribute string that holds an `&` which does not start an entity (`P&L`, `a&b`) parses in a `.tsx` file.
- A run that meets several unparseable, oversized or unreadable files names each of them, one line per file in path order, instead of stopping at the first. One refused file still gives the same single error as before, and the language server drops the refused files one at a time as before.
