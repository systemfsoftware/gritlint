# @systemfsoftware/gritlint

The npm launcher for `gritlint`, the Rust CLI that evaluates GritQL rule packs
over a repository's config files.

## Install

The package is the tarball the flake builds:

```sh
nix build github:systemfsoftware/gritlint#gritlint-tarball
```

That links `result` to `gritlint-<version>.tgz` in the Nix store. Install it
from that path, which needs no registry and works offline:

```sh
pnpm add file:$(readlink -f result)
```

The tarball carries a static `linux-x64` and `linux-arm64` binary beside a node
launcher, and `node_modules/.bin/gritlint` runs the binary for the host with the
arguments it was given, propagating its exit code. The launcher is plain node
with no dependency, and there is no postinstall step: nothing is downloaded or
compiled at install time.

On any other platform the launcher starts no binary and exits 2, gritlint's
refusal code, with:

```
gritlint: no binary for <platform>-<arch>; this package ships linux-x64 and linux-arm64
```

The manifest declares no `os` or `cpu`, so the install itself succeeds on every
platform and only running refuses.

`configuration_schema.json` in this package is the JSON Schema for
`gritlint.json`, generated from the CLI's config types. `report_schema.json` is
the JSON Schema for `gritlint check --format json` (`gritlint schema --report`).

## Rule files

Every rule file, bundled or listed under `rules` in `gritlint.json`, opens with
a `---` block of single-line `key: value` pairs, and these three keys are the
only ones allowed:

```grit
---
reasonCode: missing-provenance
nextAction: Set publishConfig.provenance to true.
locate: `"provenance": $value` where { $value <: not `true` }
---
language json
...
```

- `reasonCode` (required) is lowercase kebab-case, stable for the rule, so an
  agent can act on it.
- `nextAction` (required) is one imperative sentence: what to change.
- `locate` (optional) is a single-file GritQL pattern in the rule's language. A
  multifile rule's match has no range of its own; after the scan, `locate` runs
  over the files the match joined, in path order, and the finding takes the
  earliest range in the first file it matches. When it matches nothing, the
  finding names the first matched file with no range.

A missing required key, any other key (including a `title:` an earlier version
ignored), a repeated key, an empty value, or a `reasonCode` that is not
kebab-case refuses the run with exit 2, naming the file and the key.

`gritlint test` runs each rule's `fixtures/<rule>/bad/*` and `good/*` cases.
Every bad case commits `expected-locations.txt`, one line per finding:
`<ruleId> <path>` or `<ruleId> <path>:<line>:<column>-<line>:<column>`, and the
reported locations must equal it. A rule that declares `locate` needs at least
one bad case asserting a range.

## Output formats

`gritlint check --format <format>`; exit 0 clean, 1 findings, 2 refused.

A target file the engine cannot scan is never skipped: the run is refused, and
stderr carries one line with a next action, the same in every format:

| Refusal | Stderr line |
| --- | --- |
| the file does not parse | `ERROR gritlint: <file>:<line>:<column>: target-does-not-parse: …` |
| the file is over the engine's 1,000,000-byte limit | `ERROR gritlint: <file>: target-too-large: …` |
| the engine logged anything else for the file | `ERROR gritlint: <file>: target-not-scanned: …` |

| Format | Output |
| --- | --- |
| `human` (default) | every finding and the per-rule counts |
| `json` | `{ schemaVersion: 1, findings, summary }`; each finding carries `ruleId`, `reasonCode`, `nextAction`, `message` and a `location` (`path`, and a one-based `range` when the engine reported one; `null` names the file alone) |
| `concise` | at most 20 findings of at most 200 bytes a line, the count of the rest, and the verdict |
| `github` | one `::error` workflow command per finding, with `file`, the range when present, and the rule and reason as `title`; run it from the repository root, since GitHub resolves `file` against it |
| `github-summary` | Markdown for `$GITHUB_STEP_SUMMARY`: the verdict, at most 100 findings and the per-rule counts |

## Releasing

One version applies to the launcher and the Rust crates, and the release tooling
keeps them in step.

1. Land a change intent under `.changeset/`: a markdown file naming the bump and
   a summary. A pull request that changes `crates/`, `apps/` or `packs/` must add
   one, and `ci.yml`'s `changeset-intent` job fails the PR otherwise.
2. The reusable workflows of [`systemfsoftware/pnpm-release-management`](https://github.com/systemfsoftware/pnpm-release-management)
   collect the accumulated intents, cut the versions and open the release PR.
   Its `cargo` surface rewrites the root `Cargo.toml` and `Cargo.lock` alongside
   `npm/gritlint/package.json`, so the crates and the launcher move together in
   one reviewed commit.
3. On merge, the release workflow tags the version and cuts its GitHub Release
   from the authored changelog.

Nothing in this repository publishes to an npm registry: the suite is
distributed as this repository's flake outputs and the tarball the flake builds.
