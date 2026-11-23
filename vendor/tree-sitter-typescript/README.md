# tree-sitter-typescript (engine copy, a scanner change and four grammar changes)

This is the `tree-sitter-typescript` 0.20.5 crate that the pinned GritQL engine
builds from `resources/language-metavariables/tree-sitter-typescript`, at the
engine rev in `Cargo.lock`. The root `Cargo.toml` patches the engine's copy with
this one. Apart from this README, every file is byte-identical to the engine's
except `common/scanner.h`, and the generated `src/` of `typescript` and `tsx`
(`grammar.json`, `node-types.json`, `parser.c`, `tree_sitter/*.h`).

## The change

The external scanner refused to insert an automatic semicolon before a `<` that
starts a new line. Inside an object type or an interface, such a `<` opens the
type parameters of the next call signature, so a member list without
separators failed to parse:

```ts
export const withMaxDepth: {
  (limit: number): <A>(effect: A) => A
  <A>(effect: A, limit: number): A
} = dual(2, apply)
```

That is Effect's `dual` overload shape, and the error stopped every scan that
parses the module. The scanner now treats `<` like `(` and `[`: it inserts the
semicolon unless a binary operator is valid at that point, which is how the
grammar tells an expression from a type. TypeScript reads the source the same
way. It never takes type arguments across a line break. Expressions such as
`a\n< b` still parse as one comparison.

## The grammar changes

Four valid TypeScript constructs failed to parse, and one such file stopped every
scan of the tree that held it:

- `<out T, in U, in out W>`: a type parameter with a variance annotation.
- `import('x').T`, `import('x').A.B.C<A>` and `import('x', { with: ... }).T`: an
  `import()` type with an optional attributes object, a qualified member name and
  type arguments. It is a new `import_type` node (`argument`, repeated `name`,
  optional `type_arguments`). A type assertion that starts with one
  (`<import('x').T>y`) is not parsed.
- `export type * from 'x'` and `export type * as ns from 'x'`.
- JSX text and attribute strings that hold an `&` that does not start an entity
  (`P&L`, `a&b`): the entity token needs its closing `;`, and a bare `&` was
  neither text nor an entity.

`grammar.js` is not the source for these: the engine's `grammar.js` builds only
against a `tree-sitter-javascript` revision that is no longer published, so the
changes are made to the resolved `src/grammar.json` of both parsers by
`patch-grammar.py` (idempotent), and each `src/` is regenerated from that file
with `tree-sitter generate --abi 14 src/grammar.json` (tree-sitter 0.27 CLI; the
runtime stays `~0.20`, which loads ABI 14). Both parsers get the same edits, and
the `&` change applies to `tsx` only. The edits are:

- `type_parameter`: `choice('in', 'out', seq('in', 'out'), blank)` after the
  optional `const`.
- `export_statement`: two alternatives, `export type * <from clause>` and
  `export type <namespace export> <from clause>`.
- `import_type` (a right-associative rule, defined in `patch-grammar.py`) added to `_primary_type`, with the
  conflict `[primary_expression, import_type]` for `<import('x')>` against `x as
  import('x').T`.
- `tsx` only: `jsx_text` also accepts a lone `&`, and a JSX string accepts a lone
  `&` as an immediate `string_fragment`, so an entity (`&amp;`) still wins as the
  longer match.

The engine's own validation of rule field names reads its embedded
`tsx-node-types.json`, which does not list `import_type`, so a rule matches an
`import_type` node by its text and cannot name its `argument` field.

## Re-pinning the engine

When `Cargo.lock` moves to another GritQL rev, copy that rev's
`resources/language-metavariables/tree-sitter-typescript` `Cargo.toml`, the
files its `include` lists, and `LICENSE` over this directory, then
re-apply the `common/scanner.h` change and the grammar edits above, then regenerate both parsers. The flake check `gritlint-vendor-sync`
(`nix/gritlint-checks.nix`) fails naming every file left stale.

If the new rev already inserts the semicolon before `<` in a type, remove the
copy: this directory, the root `Cargo.toml`'s `[patch]` entry and its workspace
`exclude`, in `nix/gritlint.nix` the `../vendor` source entry, `grammar` and
`extraDummyScript`, the `gritlint-vendor-sync` check, and its bullet in
`docs/solutions/tooling-decisions/gritlint-conventions-platform.md`. Then run
`cargo update -p tree-sitter-typescript`.
