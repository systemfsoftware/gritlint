# build-output-imports

One rule, `dist-import-specifier`: a module specifier must not reach into a
package's build output by path. It judges static `import`, `export ... from`,
dynamic `import()` and `import x = require()`, type-only forms included, and fails:

- a relative specifier that reaches a `dist` segment through one or more named
  directories (`./packages/x/dist/y.js`, `../other/dist/index.js`);
- an absolute specifier with a `dist` segment;
- a package specifier whose subpath has a `dist` segment (`pkg/dist/y.js`,
  `@scope/pkg/dist/y.js`).

Import the package by its name and a subpath its `exports` map declares instead.

A relative path whose first named segment is `dist` (`./dist/x.js`,
`../dist/x.js`) is the importing package's own build output and is not judged. A string that is not a module
specifier (a bundler `outDir`, a `new URL()` argument, a `vi.mock()` call, a
comment) is not judged either.

## Enable it

```json
{
  "packs": {
    "build-output-imports": {}
  }
}
```

No parameters.
