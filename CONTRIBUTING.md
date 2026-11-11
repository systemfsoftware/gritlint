# Contributing

gritlint is a Rust workspace (`apps/gritlint` for the CLI, `crates/gritlint_core` for the engine, `crates/gritlint_bench` for benchmarks) with the rule packs under `packs/`. Rust 1.90 or newer is required.

## Setup

The flake provides the toolchain, including Rust, cargo-deny, cargo-nextest, Node 24 and pnpm 12:

```bash
git clone https://github.com/systemfsoftware/gritlint.git
cd gritlint
nix develop
```

## Test

```bash
cargo nextest run   # the Rust test suite
cargo deny check    # licences, advisories and sources (deny.toml)
```

The flake's `checks` output holds the gate: the sandboxed CLI, static checks, the tarball journey and the vendor checks. Run `nix flake check` before you open a pull request.

## Layout

- `apps/gritlint` is the CLI. Its subcommands are defined in `apps/gritlint/src/main.rs`.
- `crates/gritlint_core` holds the engine adapter, rule loading, scanning and reporting.
- `packs/<name>/` holds one pack: `pack.json`, its `rules/`, its `fixtures/`, and a README that lists its rules.
- `vendor/` holds vendored sources, including `tree-sitter-typescript`, which is excluded from the workspace.
- `nix/` holds the flake modules for the packages and checks.
- `npm/gritlint/` is the npm launcher. Its tarball is built with `nix build .#gritlint-tarball`.
- `docs/` holds plans and solutions.

## Adding a rule

A rule belongs to a pack. Add the rule under the pack's `rules/` and its fixtures under `fixtures/`, then list it in the pack's README. A pack's fixtures are its tests: `gritlint test` runs them against the rules.
