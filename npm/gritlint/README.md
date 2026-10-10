# @systemfsoftware/gritlint

The npm launcher for `gritlint`, the Rust CLI that evaluates GritQL rule packs
over a repository's config files.

## Install

```sh
npm i -D @systemfsoftware/gritlint
npx gritlint --help
```

The launcher ships no binary of its own. The matching platform package
(`@systemfsoftware/gritlint-linux-x64`, `-linux-arm64`, `-win32-x64`) is an
`optionalDependency` pinned to the same exact version, so npm installs the
right binary for the host and the launcher executes it with the arguments it
was given, propagating the exit code.

There is **no postinstall step**: nothing is downloaded or compiled at install
time. Installation flags that drop optional dependencies (`--no-optional`,
`--ignore-scripts`) leave the launcher without a binary, and it then reports the
missing platform package by name instead of failing silently.

The committed manifest deliberately carries **no `optionalDependencies`**: a
committed pin on a package that does not exist yet breaks `pnpm install` for
everyone in the workspace (pnpm#3960). `scripts/tools/gritlint/sync-version.ts
--pins` injects the three exact pins into the manifest of the publish checkout
only; they exist on npm, never in the repository.

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

A target file that does not parse is never skipped: the run is refused, and
stderr carries `ERROR gritlint: <file>:<line>:<column>: target-does-not-parse: …`
with a next action, in every format.

| Format | Output |
| --- | --- |
| `human` (default) | every finding and the per-rule counts |
| `json` | `{ schemaVersion: 1, findings, summary }`; each finding carries `ruleId`, `reasonCode`, `nextAction`, `message` and a `location` (`path`, and a one-based `range` when the engine reported one; `null` names the file alone) |
| `concise` | at most 20 findings of at most 200 bytes a line, the count of the rest, and the verdict |
| `github` | one `::error` workflow command per finding, with `file`, the range when present, and the rule and reason as `title`; run it from the repository root, since GitHub resolves `file` against it |
| `github-summary` | Markdown for `$GITHUB_STEP_SUMMARY`: the verdict, at most 100 findings and the per-rule counts |

## Releasing

This launcher rides the repository's release pipeline; it is not released on its
own. One version applies to the Rust crates, this launcher and the generated
platform packages, and **the launcher's version is the source of that version**.

1. Land a change intent for `@systemfsoftware/gritlint` (`pnpm change --bump
   minor --summary "..."`) — the launcher is a workspace package like any
   other, so `scripts/tools/plan-release.ts` plans its release with the rest of
   the set.
2. The release phase opens the Release PR; `pnpm version -r` bumps this
   manifest, and `scripts/tools/gritlint/sync-version.ts --cargo` writes the
   same version into `[workspace.package] version` in the root `Cargo.toml`, so
   the crates and the launcher move together in one reviewed commit.
3. On merge, `.github/workflows/release.yml` writes a `<pkg>@vX.Y.Z` git tag for
   each unreleased workspace version and cuts its GitHub Release from the
   authored changelog. There is no npm publish step: the suite is distributed as
   this repository's Nix flake outputs, consumed from a git ref (pinned by
   `flake.lock` rev + narHash, bwrap-sandboxed), so `gritlint` is the flake-built
   Rust CLI (`bin/gritlint`) rather than an npm download.

Nothing in this repository publishes to an npm registry: there is no npm token,
no OIDC trusted publishing, and no trusted-publisher bootstrap.
