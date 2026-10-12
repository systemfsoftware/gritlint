# tree-sitter-typescript (engine copy, one scanner change)

This is the `tree-sitter-typescript` 0.20.5 crate that the pinned GritQL engine
builds from `resources/language-metavariables/tree-sitter-typescript`, at the
engine rev in `Cargo.lock`. The root `Cargo.toml` patches the engine's copy with
this one. Apart from this README, every file is byte-identical to the engine's
except `common/scanner.h`.

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

`parser.c` is generated from `grammar.js`, which is unchanged, so neither
parser is regenerated.

## Re-pinning the engine

When `Cargo.lock` moves to another GritQL rev, copy that rev's
`resources/language-metavariables/tree-sitter-typescript` `Cargo.toml`, the
files its `include` lists, and `LICENSE` over this directory, then
re-apply the `common/scanner.h` change. The flake check `gritlint-vendor-sync`
(`nix/gritlint-checks.nix`) fails naming every file left stale.

If the new rev already inserts the semicolon before `<` in a type, remove the
copy: this directory, the root `Cargo.toml`'s `[patch]` entry and its workspace
`exclude`, in `nix/gritlint.nix` the `../vendor` source entry, `grammar` and
`extraDummyScript`, the `gritlint-vendor-sync` check, and its bullet in
`docs/solutions/tooling-decisions/gritlint-conventions-platform.md`. Then run
`cargo update -p tree-sitter-typescript`.
