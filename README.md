# gritlint

[![License: MIT](https://img.shields.io/badge/license-MIT-blue?style=flat-square)](LICENSE)

gritlint runs [GritQL](https://github.com/biomejs/gritql) rule packs over a repository's config files and reports every violation with its reason and the next action. A pack is a set of rules that encode one convention, such as where a service module may hold a driver, or how a public npm package must publish. You enable the packs you want and gritlint checks them.

It is a single Rust binary. The GritQL engine is Biome's.

## Quick start

Build the CLI from the flake, then check a repository:

```bash
nix build github:systemfsoftware/gritlint#gritlint
./result/bin/gritlint check .
```

Enable packs in `gritlint.json` at the repository root. A pack that takes no parameters is enabled with an empty object:

```json
{
  "packs": {
    "cell-architecture": {}
  }
}
```

`gritlint check` reads the packs from that file and scans the tree from the path you give it.

## Packs

These packs ship with the CLI:

| Pack | Enforces |
| :-- | :-- |
| `cell-architecture` | A `*.service.ts` module holds no driver and exports no Layer value. |
| `source-resolution` | A workspace package's name resolves to its `src/` for workspace tools and to `dist/` for registry consumers, across the export map, tsconfig, Vite and api-extractor. |
| `typecheck-build-mode` | A reference-only tsconfig root's `typecheck` script runs `tsc -b`, so it reaches the referenced projects. |
| `npm-provenance` | Public npm packages publish with trusted publishing (OIDC) and provenance. |

Each pack's README lists its rules and its parameters.

## Commands

| Command | What it does |
| :-- | :-- |
| `gritlint check [root]` | Scan a tree and report findings. |
| `gritlint test` | Run a pack's fixtures against its rules. |
| `gritlint lsp` | Serve findings to an editor over the Language Server Protocol on stdio. |
| `gritlint export biome` | Print a rule as a Biome GritQL plugin, or list which rules export. |

`check` takes these flags:

| Flag | Meaning |
| :-- | :-- |
| `--config <PATH>` | Use this config file instead of `gritlint.json` at the root. |
| `--rules <PATH>` | Load extra rules from this path. Repeatable. |
| `--format <FORMAT>` | `human` (default), `json`, `concise`, `github` or `github-summary`. |

`human` lists every finding with its reason code and next action, and the count per rule. `json` writes the versioned report. `github` writes one GitHub Actions `::error` annotation per finding, and `github-summary` writes Markdown for `$GITHUB_STEP_SUMMARY`.

## Install

Install from the flake, which is the supported route:

```bash
nix build github:systemfsoftware/gritlint#gritlint
```

Or take `packages.<system>.gritlint` from the flake as an input of your own flake.

## Contributing

Development setup, the test commands and the layout are in [CONTRIBUTING.md](CONTRIBUTING.md).

## License

MIT. See [LICENSE](LICENSE).
