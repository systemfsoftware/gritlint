---
"@systemfsoftware/gritlint": patch
---

An `import()` type now parses with a qualified member name (`import('react').JSX.Element`) and with an attributes object (`import('x', { with: { 'resolution-mode': 'import' } }).T`), and `ts-extension-specifier` and `dist-import-specifier` read only its module string, not a string in its attributes or type arguments.
