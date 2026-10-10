---
title: gritlint as a standalone repository - Plan
type: feat
date: 2026-10-10
topic: gritlint-standalone
supersedes: docs/plans/2026-10-10-0913-feat-gritlint-standalone-plan.md
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-brainstorm
execution: code
source_commit: systemfsoftware/systemfsoftware@df908ddfe497e53c1c03b92519a96c913c441a58
---

# gritlint as a standalone repository - Plan

## Goal Capsule

- **Objective:** gritlint lives in `systemfsoftware/gritlint`. A consumer switches by changing only its flake input URL, and later PRs make the tool measurably faster, agent-readable, fully tested, installable from a tarball, and usable from an editor.
- **Means:** seven stacked PRs on trunk `main`. PR1 is a faithful import of `systemfsoftware/systemfsoftware@df908ddfe497e53c1c03b92519a96c913c441a58` plus the flake, MIT licensing, the Rust gate and the dogfood conventions gate in CI. PR2 to PR7 deliver Unit B1 Done 7, 2, 3, 4, 5 and 6, in that order.
- **Product authority:** the Unit B1 round-3 contract, then the session rulings recorded as `session-settled` entries, then this plan, then planner judgment. Removing gritlint from the monorepo belongs to a later unit, which starts when PR1 merges.
- **Stop conditions:** stop and ask before adding any third-party executable, GitHub Action or library crate outside R11 and the approved list in KTD9; before copying any monorepo workflow, tooling or pack; before referencing a private repository, a secret or an environment; and before any push whose ancestry contains a monorepo commit.
- **Execution profile:** PR1 is planned to unit level and executed now. PR2 to PR7 are planned to unit level with test scenarios here, and each gets a ce-plan deepening pass when it starts, because each depends on numbers or code the earlier PR produces.
- **Open blockers:** none for PR1. Q5 to Q9 must be answered before PR5 work, Q2 and Q10 before PR6, and Q1 and Q11 before PR7.

---

## Product Contract

### Summary

Import gritlint unchanged, then rebuild what is unproven: per-run compilation, output shape, rule test coverage, packaging and editor integration. The import PR is small so the monorepo-removal unit can start early.

### Problem Frame

gritlint was built inside the monorepo, and its quality was never measured there. Each rule is compiled again for every directory it runs in (`decide.rs` `run_directory` -> `execute` -> `engine::run` -> `src_to_problem_libs`). A finding with no engine range is reported at line 1 (`decide.rs` `first_match` and `finding_for`). The JSON output has no `schemaVersion` and no reason codes. The npm wrapper (`npm/gritlint/package.json`) is a private stub with no `bin`. Nothing has been benchmarked. Consumers reach gritlint through the monorepo flake, which couples their lockfile to the whole monorepo.

GritQL stays the engine. Its multifile patterns and offline execution under bubblewrap are what gritlint offers that single-file engines (Biome plugins, ast-grep rules) do not.

### Key Decisions

- **Import from `df908ddf`, the monorepo's `main` tip.** (session-settled: user-directed — chosen over `cf6ab028` and `6a7c247b`: its gritlint paths equal `6a7c247b`'s, which add `packs/cell-architecture` and the matching `configuration_schema.json`.) No `.changeset/` intent file is imported. Governs R1.
- **PR1 is the faithful import; improvements come later.** (session-settled: user-directed — chosen over one large PR: the monorepo-removal unit waits on PR1.) Governs R1 to R11.
- **Stay on the GritQL crates, tracking `biomejs/gritql`.** (session-settled: user-directed — chosen over Biome plugins and ast-grep, which are single-file.) The crates stay a pinned git dependency and are never vendored. Governs R3.
- **Leave out of the import what has no consumer here.** `scripts/tools/gritlint/sync-version.ts` is only run by the monorepo's `gate:repo` selftest and its `--pins` mode feeds the retired npm platform packages; the release tooling's `cargo` surface writes `Cargo.toml` and `Cargo.lock` instead (PR6). The monorepo's origin plan and its Windows MAX_PATH solution describe monorepo history and a Windows npm publish this repo refuses to build. `docs/solutions/tooling-decisions/gritlint-conventions-platform.md` comes over as the design record. Governs R1.
- **Stack every PR on the previous one.** Every PR edits `flake.nix` or the CI workflow, so sibling PRs off `main` would conflict. PR1 merges first; GitHub retargets the rest. Governs the PR sequence.
- **Order the later PRs so each one makes the next cheaper or safer.** CI hygiene (PR2) comes first so later PRs run on cached builds. Performance (PR3) comes before output (PR4) and the fixture harness (PR5) because it rewrites the code they snapshot. Distribution (PR6) needs the final CLI. The editor surfaces (PR7) reuse PR4's findings.
- **Write this repo's own CI workflows.** At `df908ddf` the monorepo has no reusable Rust or Nix workflow: `reusable-checks.yml`, `reusable-smoke.yml` and `reusable-race.yml` drive pnpm workspaces. PR6 consumes `pnpm-release-management`'s reusable `release.yml` and `changeset-check.yml` at `8cd6e83009531de040cd9c03e1370b4966fd2a12`, the revision the monorepo's `main` calls at `df908ddf`. If O1 publishes a Rust or Nix workflow later, a follow-up adopts it pinned by SHA.
- **Ship release wiring with distribution (PR6), not PR1.** (session-settled: user-directed — still in force: drop `private` on `npm/gritlint`, changesets and GitHub Releases through the reusable workflow, no npm registry, and a flake check that no release output shadows `packages.<system>.gritlint`.) Until the tarball has a `bin` there is nothing to release.
- **Release through the monorepo's working caller, unchanged.** (session-settled: user-directed — chosen over the later `bc3b9b37` revision: `df908ddf`'s Release run succeeded on `8cd6e830`.) `release.yml` calls `systemfsoftware/pnpm-release-management/.github/workflows/release.yml@8cd6e83009531de040cd9c03e1370b4966fd2a12` with `ci-workflow: ci.yml` and the job permissions `contents: write`, `pull-requests: write`, `actions: write`, as `df908ddf:.github/workflows/release.yml` does; `changeset-check.yml` calls the sibling `changeset-check.yml` at the same SHA. Both are consumed from the org and never copied. No repository-settings change is a dependency: if the release workflow needs "Allow GitHub Actions to create and approve pull requests", PR6's description says so in one line, and nothing else changes. Governs R24.
- **R20 measures cases against rule defects; cargo-mutants measures the Rust.** cargo-mutants mutates Rust, and a rule is GritQL inside Markdown, so no Rust mutant stands for a rule clause. Done 4's two parts each get their own evidence: rule cases against one-step rule defects (R20), Rust mutants against the harness and engine that run them (R21). Governs R20, R21.

### Requirements

**PR1 — import, identity and licence**

- R1. One commit adds exactly the files in Appendix A at the same paths, byte-identical to `df908ddf`, and cites that SHA. Later PR1 commits change only the files in Appendix B and add only the files in Appendix C.
- R2. No commit reachable from any pushed ref has a monorepo commit in its ancestry.
- R3. GritQL stays a git dependency on `biomejs/gritql` rev `4ca283484ab9bd11ca3cbc9e14b5ad2db5d61a37`.
- R4. Every licence surface is MIT under "System F Software": root `LICENSE`, `Cargo.toml`, `npm/gritlint/LICENSE`, and later `npm/gritlint/package.json`. Both LICENSE files carry the GritQL copyright notices, because the binary links GritQL statically.

**PR1 — flake contract**

- R5. `packages.<system>.gritlint` is the bubblewrap-sandboxed CLI and `packages.<system>.gritlint-unwrapped` is the plain build, for `x86_64-linux` and `aarch64-linux`. No darwin outputs.
- R6. Evaluating any output for `x86_64-windows` throws a message that names gritlint and both supported systems.
- R7. `nix flake check` builds the sandboxed package and fails when `packages.<system>.gritlint` stops isolating a run (it reads outside the working directory, writes the working directory, or keeps the caller's environment), when that name is bound to anything but the sandbox wrapper, or when `packages.<system>.gritlint-unwrapped` is not the compiled binary.

**PR1 — gates**

- R8. Rust gate: `scripts/tools/gate-rust.sh` runs unchanged on every PR and on `main`. It covers fmt, clippy `-D warnings`, the `decide.rs` loop check, the workspace tests, `gritlint test packs`, schema drift, the network-feature check and `cargo deny`.
- R9. Dogfood: `gritlint.json` names this repository in `npm-provenance.repositoryUrl`. CI shows the sandboxed binary exiting 1 on a planted `typecheck-build-mode` violation and 0 once fixed, exiting 1 under this repository's `gritlint.json` on a published package whose `repository.url` names the monorepo and 0 once it names this repository, each failure naming its rule, then `./bin/gritlint check` passing on this repository.
- R10. Nix lane: on `x86_64-linux` and `aarch64-linux` hosted runners, the R6 refusal holds and every flake output for that system builds.
- R11. CI uses standard hosted runners only, no secrets, environments or private repositories, every action pinned by commit SHA, and no executable beyond Nix, bubblewrap, the Rust toolchain and cargo-deny in PR1. Later PRs may add only the KTD9 tools.

**PR2 — Rust hygiene and CI cost (Done 7)**

- R12. The flake builds with crane, so dependency artifacts are a separate cached derivation. CI restores and saves Nix store paths without secrets.
- R13. Every check `scripts/tools/gate-rust.sh` runs becomes a flake check that each Nix job builds, except `cargo deny check`, which needs the network for advisories and stays a CI step. Tests run under cargo-nextest. `rust-version` in `Cargo.toml` is the pinned MSRV, and a check compiles the workspace with exactly that toolchain.
- R14. CI cost is measured on exact runs through the GitHub REST API. Each run records `billable.total_ms` from `/actions/runs/{id}/timing`, which is 0 on public repositories, and its job minutes: each job's `completed_at - started_at` from `/actions/runs/{id}/attempts/{n}/jobs`, rounded up to a whole minute, which is how GitHub bills a private repository. A cold run restores no cache. A warm run is a re-run attempt of the same run, after the first attempt saved its caches. PR2 measures a cold and a warm run of its own, PR1's run 38021222065 (cold) and a re-run attempt of it (warm), and the monorepo's gritlint lanes at `df908ddf` (the `rust` and `gritlint` jobs of CI run 38008617968 and both jobs of Nix run 38008617804, a warm `main` push). PR2's warm run costs fewer job minutes than PR1's warm run and the monorepo's, and the PR body records every number.

**PR3 — performance (Done 2)**

- R15. A run compiles each enabled rule once and makes one multithreaded pass over the selected files. The findings are the same as before for every input.
- R16. An in-repo Rust benchmark harness runs gritlint over a pinned public TypeScript corpus of at least 100k lines, which Nix fetches by hash and the repository never vendors. CI runs it on every PR and reports wall time for the PR1 binary and the current binary in the step summary.
- R17. The speedup on the corpus is at least 10x, or the PR states the measured ceiling and what bounds it.

**PR4 — agent-first output (Done 3)**

- R18. `--format json` emits a document with `schemaVersion`. Each finding carries `ruleId`, `reasonCode`, `nextAction`, and a location: the path, plus the start and end line and column of the violating node's range as the engine reported it in that file; for a multifile rule that range comes from the rule's locator (KTD22). The location is the file alone only when the violation is an absent key, so the engine has no node and the locator matches nothing. A finding is never placed on a line neither the engine nor the rule's `locate` pattern reported. *(Changed in PR4 planning; Q4 resolved: per-rule locators with committed expected locations.)*
- R19. A bounded concise reporter prints at most a fixed number of findings and a count of the rest. `--format github` emits one annotation per finding, and `--format github-summary` emits the step summary that the CI step appends.

**PR5 — rule tests and mutation testing (Done 4)**

- R20. Every pack rule has at least one valid and one invalid case, run by `gritlint test packs` against each bad case's committed `expected-locations.txt`. For each one-step defect in a rule's GritQL (a dropped conjunct or `or` branch, `or` turned to `and`, a negated predicate, a widened match or regex, a `locate` branch on the wrong node), at least one case fails. The exception is a defect no input can tell apart from the correct rule, and Appendix E names each such clause and why. A case that no such defect can fail is deleted.
- R21. cargo-mutants runs in CI on pushes to `main` only, over `gritlint_core`. It never runs on a pull request, and no mutant is built or run locally. Each run's caught, missed, timeout and unviable counts and its missed list reach the step summary; the first `main` run's report is cited in a follow-up.
- R27. Every pull-request CI job finishes within 10 minutes, from PR5 on.
- R28. Every bundled case also runs under `cargo test`, so a Rust mutant that changes a bundled case's outcome is caught.

**PR6 — distribution and release (Done 5)**

- R22. The flake builds an npm-style tarball with a `bin` entry and binaries for `linux-x64` and `linux-arm64` that run outside Nix. Consumers install it from the tarball, never from an npm registry.
- R23. A CI consumer journey installs the tarball into a scratch project and runs `gritlint check` there.
- R24. `npm/gritlint` loses `private`. Changesets and GitHub Releases run through `pnpm-release-management`'s reusable workflows at `8cd6e830`, a changeset check guards `npm/gritlint/`, and a flake check fails if any release output shadows `packages.<system>.gritlint`.

**PR7 — editor and Biome surfaces (Done 6)**

- R25. `gritlint lsp` speaks the Language Server Protocol over stdio and publishes diagnostics for open files covered by the enabled packs.
- R26. `gritlint export biome` writes Biome GritQL plugin files for each single-file rule in a language Biome plugins support, and refuses every other rule with a reason code.

### Acceptance Examples

- AE1. **Covers R5.** **Given** a scratch consumer flake that builds `packages.x86_64-linux.gritlint` from `github:systemfsoftware/systemfsoftware`, **when** only its input URL changes to `github:systemfsoftware/gritlint/<PR1 branch>`, **then** the same attribute builds and `gritlint --version` prints `0.1.0`.
- AE2. **Covers R6.** **Given** this flake, **when** `nix eval --raw .#packages.x86_64-windows.gritlint` runs, **then** it exits non-zero and stderr names gritlint, `x86_64-linux` and `aarch64-linux`.
- AE3. **Covers R1.** **Given** `df908ddf` fetched into the clone, **when** every Appendix A path outside Appendix B is compared by blob hash and mode with the import commit, **then** all match.
- AE4. **Covers R7.** **Given** the flake, **when** `nix/gritlint-sandbox.nix` drops bubblewrap, binds the working directory writable, binds the host root, or keeps the environment, or `packages.<system>.gritlint` is rebound to `gritlint-unwrapped`, or `gritlint-unwrapped` to the wrapper, **then** `nix flake check` fails.
- AE5. **Covers R9.** **Given** `npm-provenance.repositoryUrl` set to this repo, **when** `./bin/gritlint check` runs at the root, **then** it exits 0.
- AE6. **Covers R15, R16.** **Given** the pinned corpus, **when** the benchmark runs the PR1 binary and the PR3 binary, **then** both produce the same findings and the step summary shows both wall times.
- AE7. **Covers R18.** **Given** a bundled multifile rule whose bad fixture case has the violating node present, **when** gritlint reports it in JSON, **then** the location is that node's range, equal to the case's committed expected location; **given** a case whose violation is an absent key, the location is the matched file alone with `range` null. No finding is placed at line 1 unless the engine reported line 1.
- AE8. **Covers R23.** **Given** the PR6 tarball, **when** a scratch project installs it from the file and runs `gritlint check` with a planted violation, **then** it exits 1 with the finding.

### Scope Boundaries

- Removing gritlint from the monorepo and repointing consumers.
- Repository settings, including "Allow GitHub Actions to create and approve pull requests".
- darwin and Windows builds.
- Publishing to any npm registry.
- New packs, and changing what any existing rule matches, except where a PR5 case exposes a defect or a Q5, Q6 or Q9 ruling decides the clause (U12).
- Monorepo tooling: TypeScript toolchain, oxlint, dprint, comment-checker, the consumer-store lib, `deno-tool`, `repo-checks`, `test-timings`, and every monorepo workflow.

### Dependencies / Assumptions

- `df908ddf` is the monorepo's `main` at 2026-10-10 00:20 UTC. Its gritlint paths, `flake.lock`, `.github/actions/checks-lane/action.yml` and `.github/workflows/nix.yml` are identical to `6a7c247b`'s.
- The monorepo's locked `nixpkgs` is `aa48d347080940b8a2b8d2f48228674e280a3514` and `rust-overlay` is `368fee9beaab04ca6fe7af28db63caa9badb22fa`. `nix/gritlint.nix`'s `cargoHash` was produced by that nixpkgs.
- GritQL's `Problem::execute_files` takes `&self`, so one compiled problem can run over many file sets, and it already runs single-file patterns over files in parallel with rayon (`crates/core/src/problem.rs:318`, `:488-500` at `4ca28348`). Multifile patterns run sequentially within one call.
- At `4ca28348`, a multifile pattern's match carries no top-level range, and its variable ranges are shared by every file the execution bound (`crates/core/src/marzano_context.rs:226`: "todo, for multifile we need to split up the matches by file"). All 15 pack rules are multifile. Measured in PR4 planning: the pack bad fixtures produce 25 engine matches, all with an empty range list. A single-file pattern's match carries exact ranges: a scratch `language json` rule `` `"url": $_` `` reported the `url` line of a `package.json`.
- Biome plugins are `.grit` files that call `register_diagnostic` and declare a target language (biomejs.dev/linter/plugins). Which Biome release added JSON targets is checked in PR7.

### Outstanding Questions

- Q1. *(Resolve Before PR7)* Verifying the Biome export by running Biome needs Biome as a third-party executable, which is not on the approved list. Without approval, PR7 checks exported plugins by compiling them with gritlint's own GritQL engine plus a stub `register_diagnostic`, which proves syntax but not Biome's acceptance.
- Q2. *(Resolve Before PR6)* A tarball binary that runs outside Nix must be statically linked (musl). If crane cannot cross-build the `aarch64-linux` musl binary on the `x86_64-linux` runner, PR6 needs the tarball assembled from both runners' outputs, which turns the tarball into a CI artifact rather than a single flake output. Deferred to PR6 planning; ask only if neither shape works.
- Q3. *(Resolved in PR3 planning)* The PR3 corpus is `Effect-TS/effect` at `0b17b9e2e43c2f95726470aee188fd9e2bf1baf6` (MIT, public), fetched with hash `sha256-Cfm8ZP7F3lE+3LmO4+IuJBcUTPX+qXVyRIX4tU04bxM=`. It holds 866,330 lines of TypeScript, and gritlint selects 2,641 files from it, including 43 `package.json`, 48 `tsconfig*.json` and 7 `vitest.*` files.
- Q4. *(Resolved in PR4 planning: root ruling, per-rule locators.)* R18 as first written assumed every match has an engine range; none of the 15 bundled rules gets one (Dependencies / Assumptions). Each bundled rule declares a `locate` pattern (KTD22), and each bundled bad fixture case commits its expected location, which `gritlint test packs` checks; that pulls the location part of PR5's per-rule snapshots forward. The file-alone location stays only where the violation is an absent key. Rejected: no locators, which gives no line for any bundled rule and fails round-3 Done 3's exact location for every finding.
- Q5. *(Resolve Before PR5 work)* `source-resolution/vitest-conditions`: its `nextAction` and the pack README say to set the condition on both `resolve.conditions` and `ssr.resolve.conditions`, but the GritQL tests only the ssr list, so a config that sets only the ssr list passes. Every existing fixture sets both, so nothing pins either reading. (a) Recommended: make the rule require both lists. The `locate` pattern gains a branch for the client `conditions` pair without the condition, so a present client list keeps a range (Appendix D's rule). Add `bad/client-other-condition`, which reports that pair, and `bad/only-ssr-conditions`, where the client list is absent and the finding names the file alone. This changes what the rule reports, so consumers can see new findings, and it ships with a changeset. (b) Narrow the `nextAction` and README to the ssr list and add a good case pinning that.
- Q6. *(Resolve Before PR5 work)* Two values the rules cannot read. `tsconfig-paths`: a `paths` key that is a subpath of the package's own name (`"@acme/app/*"`). `tsdown-exports`: `exports` bound to an identifier (`exports: sharedExports`) rather than an inline object. Both are silent today. A ruling makes each a good case (pin silence) or a bad case (change the rule). Until then neither case is committed (Appendix E, GAP 7 and GAP 12).
- Q7. *(Resolve Before PR5 work)* Scope of "no local mutation". (a) Scratch red probes for rule cases: copy `packs/` to a temporary directory, apply one Appendix E defect to one rule, run `gritlint test packs`, require the named case to fail, and delete the copy. Nothing is committed and no mutation tool runs. Without (a), R20's evidence is Appendix E's written clause-to-case mapping, which is reviewed but never executed. (b) `cargo mutants --list` locally: it enumerates mutants and builds or runs none. It would let the PR cite the mutant ids in code `bundled_packs` reaches, and smoke-test the workflow's list step before the first `main` run. Recommended: allow both.
- Q8. *(Resolve Before PR5 work)* The mutants workflow. O1's workflows at `systemfsoftware/systemfsoftware@9154691d4bd5c680b8164a9e9301e77518509312` hold `reusable-mutation.yml`, and it drives Stryker through pnpm and turbo: its `working-directory` input is "The pnpm workspace root", its jobs run `corepack pnpm exec turbo run build` and restore `.stryker-incremental`. O1 has no cargo-mutants workflow. Proposal (KTD30): this repository's own `.github/workflows/mutants.yml`, push to `main` only, report-only. Missed mutants (cargo-mutants exit 2) and timeouts (exit 3) leave the run green and are listed in the step summary; a failing baseline (exit 4) or a usage error (exit 1) fails it (mutants.rs, Exit codes). Approve the repo-local workflow and its report-only status, or name a threshold that fails it.
- Q9. *(Resolve Before PR5 work)* Three clauses whose intended behaviour is unstated, so no case can be chosen. (1) `npm-provenance/provenance`: does a `provenance` key outside `publishConfig` count? It decides `$publish` against `$program` in the `provenance` branch and `locate`'s `within "publishConfig"`. (2) `typecheck-build-mode`: does `"references": []` count as a reference-only root? (3) `typecheck-build-mode`: should the `includes` test accept `tsc -b --verbose`, or only `tsc -b` and `tsc --build`? U12 commits each ruling as a good case pinning the ruled reading, or as a rule change with its bad case and a changeset.
- Q10. *(Resolve Before PR6)* PR6's release wiring against the pinned `pnpm-release-management` workflows at `8cd6e830`. Their caller contract needs a root `package.json` with `packageManager` pinning pnpm 12, a `pnpm-workspace.yaml` whose member is `npm/gritlint`, a `pnpm-lock.yaml`, and a `workspace-tarballs` flake output. Their `mkPnpmWorkspacePackages` names a member's output after the package without its scope, so `@systemfsoftware/gritlint` would become `packages.<system>.gritlint` and collide with R5's sandboxed CLI. PR6 planning decides the files and the attribute the tarball takes; R24's shadow check stays the guard. U9 lists neither yet.
- Q11. *(Resolve Before PR7)* All 15 bundled rules are multifile, so `gritlint export biome` (R26) has no bundled rule to export. PR7 planning decides whether the export snapshots and Q1's check run against a consumer rule file or a new single-file bundled rule, which Scope Boundaries would then have to permit.

<!-- ce-section: work-relationships -->
### Work Relationships

| PR | Delivers | Stacks on | Branch |
| --- | --- | --- | --- |
| PR1 | R1 to R11 (Done 1) | `main` | `feat/import-gritlint` |
| PR2 | R12 to R14 (Done 7) | PR1 | `feat/crane-ci` |
| PR3 | R15 to R17 (Done 2) | PR2 | `feat/single-pass-engine` |
| PR4 | R18, R19 (Done 3) | `main` at `1d60be3` (PR3 squash-merged) | `feat/agent-first-output` |
| PR5 | R20, R21, R27, R28 (Done 4) | PR4 (#6, head `69f7b67a`) | `feat/pack-fixture-harness` |
| PR6 | R22 to R24 (Done 5) | PR5 | `feat/tarball-release` |
| PR7 | R25, R26 (Done 6) | PR6 | `feat/lsp-biome-export` |

---

## Planning Contract

### Key Technical Decisions

- KTD1. **The flake carries only gritlint.** Inputs are `nixpkgs` and `rust-overlay`, locked to the monorepo's revisions (Dependencies / Assumptions), so `nix/gritlint.nix`'s `cargoHash` and the store paths stay valid. `forEachSystem`, `rust` and the version read from `Cargo.toml` come from the monorepo's `flake.nix:27-44`, with the refusal message naming gritlint. There is no `comment-checker` input, and no `pnpm-release-management` input until PR6. Governs R5, R6.
- KTD2. **R7's checks run the exported attributes.** `checks.<system>.gritlint-is-sandboxed` rebuilds `packages.<system>.gritlint` with `.override { gritlint = probe; }`, where the probe is a script that exits 1 naming each escape it achieves, and runs it inside the build from a working directory beside a file it must not see. The probe first runs unwrapped and must report escapes, so a probe that sees nothing fails the check. Rebinding the name to `gritlint-unwrapped` fails the `override`, which `nix/gritlint.nix` does not accept. `checks.<system>.gritlint-unwrapped-is-plain` requires `bin/gritlint` to be an ELF binary that prints `gritlint <Cargo.toml version>`. `checks.<system>.gritlint` is the package itself, because `nix flake check` evaluates packages without building them and builds only checks. Governs R7.
- KTD3. **One CI workflow, two job kinds.** `rust` runs `nix develop --command sh scripts/tools/gate-rust.sh` with the monorepo lane's cargo cache (`checks-lane/action.yml:117-145`), restored everywhere and saved on `main` only. `nix` runs per system: the refusal eval; the AppArmor user-namespace sysctl, which R7's checks need because they run bubblewrap inside the build; a build of every attribute of every flake output that has that system, with output kinds and attribute names both enumerated through `nix eval` so a new output cannot skip CI; then `scripts/tools/gate-journey.sh` and `./bin/gritlint check` against the binary it just built. Folding the journey into the Nix job reuses that build, where the monorepo's separate `gritlint` lane built it again. The closure cache keyed on the sandboxed output path (`checks-lane/action.yml:149-182`) carries over. Governs R8 to R10.
- KTD4. **Actions pinned by SHA:** `actions/checkout` v7.0.1 `3d3c42e5aac5ba805825da76410c181273ba90b1`, `cachix/install-nix-action` v31.11.1 `13d8dd58da0234aa297dedd986986ccb8e7f3e24`, `actions/cache` v6.1.0 `55cc8345863c7cc4c66a329aec7e433d2d1c52a9`. These are the actions the monorepo lanes already use. Governs R11.
- KTD5. **The dev shell holds the `rust-toolchain.toml` toolchain and `cargo-deny`.** Nothing else runs in PR1's gate. Governs R8, R11.
- KTD6. **PR1 adds no tests.** The imported suites run under R8. R7's checks and R9's journey are gates over contracts this plan states, run by `nix flake check` and CI, and each was shown to go red on a planted break. Governs R7 to R9.
- KTD7. **PR3's baseline is built from git, never checked in.** The benchmark takes `packages.x86_64-linux.gritlint-unwrapped` from `builtins.getFlake "github:systemfsoftware/gritlint/80e983b629cc939a38acad0c52d1bf4a50b1bd60"`, which is PR1's head, and the current flake's package as the candidate. Pure evaluation accepts the pinned rev, and the result is PR1's own output path (`909xaiwv…-gritlint-0.1.0`). PR1's squash merge has the same tree, so it has the same output path, and the pin never moves. Governs R16, R17.
- KTD8. **Test layers for later PRs, from the admission gate.** PR3: a differential check, PR1 binary against PR3 binary on the corpus, inside the benchmark. PR4: in-process tests through `gritlint_core`'s public API, including a regression case for the line-1 fallback. PR5: one fixture case per rule outcome, plus mutation evidence. PR6: one packed-install journey. PR7: in-process LSP tests over `lsp-server`'s in-memory connection, and export snapshots. After PR4, three runs cross a process boundary: R9's `gate-journey.sh`, KTD26's `gritlint-agent-journey` check and R23's PR6 consumer journey; KTD12's `gritlint-packs` also runs the built CLI's `test` subcommand. Governs R15 to R26.
- KTD9. **Approved tools for later PRs:** crane (a Nix library), cargo-nextest, cargo-deny, cargo-mutants (CI on `main` only), and in PR6 pnpm, node, `pnpm-release-management`'s `sandbox` and its reusable workflows at `8cd6e83009531de040cd9c03e1370b4966fd2a12`. For PR7, `lsp-server` (maintained by rust-lang in the rust-analyzer repository) and `lsp-types` (maintained by the gluon-lang organisation), each admitted through cargo-deny. Anything else stops for approval. Governs R11.
- KTD10. **crane builds over the pinned vendor tree.** `nix/gritlint.nix` keeps `rustPlatform.fetchCargoVendor` with the existing `hash`, because crane's own git vendoring would face the duplicate crate names and submodule-only crates that `gritlint.nix` documents. The vendor is fetched from a source holding only `Cargo.lock`, so a source edit leaves the vendor and both dependency derivations unchanged. A small derivation copies the vendor's `source-git-0`, adds `resources/node-types` from the pinned engine checkout beside it (replacing `preBuild`), links `source-registry-0`, and writes `config.toml` with `@vendor@` resolved. crane takes it as `cargoVendorDir`. Governs R12.
- KTD11. **Two dependency derivations.** The package builds against release-profile dependencies (fat LTO, one codegen unit). clippy, nextest and `cargo tree` share dev-profile dependencies, because linking each test binary under fat LTO would cost more than a second dependency build. The dependency derivations and the vendor tree are `passthru` on `gritlint-unwrapped`. Governs R12, R13.
- KTD12. **The Rust gate becomes flake checks.** `checks.<system>` gains `gritlint-fmt`, `gritlint-clippy` (`-D warnings`, all targets), `gritlint-nextest`, `gritlint-packs` (`gritlint test packs` with the built CLI), `gritlint-schema` (the built CLI's `schema` against `npm/gritlint/configuration_schema.json`), `gritlint-decide-loops` and `gritlint-engine-features`, each carrying `gate-rust.sh`'s logic. `checks.x86_64-linux.gritlint-msrv` runs `cargo check --workspace --all-targets` with the toolchain read from `rust-version`, on one system because language support does not vary by architecture. `gate-rust.sh`, the `rust` job and its cargo cache are deleted. `cargo deny check` runs in the `x86_64-linux` Nix job through `nix develop`, after the build has realised the dev shell. The dev shell gains cargo-nextest. Governs R13.
- KTD13. **One Nix store cache per system.** The key is a hash of every build target's output path, with that system's prefix as the restore key, so an exact hit means nothing to build and a prefix hit still carries the dependency derivations. Restore with `nix copy --no-check-sigs --from file://`. On a miss, the export starts from every path in the closure of the targets' derivations, outputs included, that has no signature and is not a `.drv` (locally built or restored), and copies those paths with their closures, because a `file://` binary cache refuses a path whose references it does not hold. Signed paths enter the bundle only as references of unsigned ones. Saving happens on every ref, because GitHub scopes caches by ref: a pull request's run can write only its own ref's cache, and `main`'s bundle is written only by `main` runs. This replaces PR1's closure cache. Governs R12.
- KTD14. **Cost from the exact runs.** `/timing` is read for every measured run and recorded as is. Job minutes come from the jobs API of the named attempt. Warm runs are produced with `gh run rerun` on the cold run, so the commit and ref stay the same. A re-run rebuilds nothing, which flatters it. The table therefore also reports, for information, the first PR2 run on a new commit after the cold run: it restores through the prefix key and recompiles only the workspace. Governs R14.
- KTD15. **PR2 adds no tests.** The KTD12 checks carry `gate-rust.sh`'s existing logic. The red probes in U5 run on scratch copies and are deleted, per the admission gate. Governs R13.
- KTD16. **Compile once, execute many.** `engine` splits `run` into `compile`, which returns a `Compiled` that owns the GritQL `Problem` and its compilation warnings, and `Compiled::execute(files)`. `load_rules` already compiles every rule once against the full rule set to validate it. It now keeps that result in `CompiledRule`, and `scan` only calls `execute`. No code path compiles during a scan, and the types enforce it, so the compile-count statistic that U6 first proposed is dropped: it would change the JSON output, which belongs to PR4. Governs R15.
- KTD17. **One rayon pass over an ordered task list.** The plan flattens into tasks: one per single-file rule batch, and one per directory batch of each multifile rule. PR1 also ran a multifile rule with no directories once over no files and discarded the result; that run could only fail by compiling the rule, which now happens at load, so it is dropped. `par_iter` maps each task to a `Result`, and the results are collected into a `Vec` in task order. Folding that `Vec` in order means the first error, and the first parse diagnostic, are the ones a sequential run would return, whatever the thread schedule. A `collect::<Result<_>>` would return whichever error a thread hit first. `canonicalize` is unchanged. The pool is rayon's global pool, so `RAYON_NUM_THREADS` sets it. Governs R15.
- KTD18. **Only the files a task evaluates are read.** The walk returns paths, and `SelectedFile` holds a language and a path. After planning, `decide::sources_needed` lists the files some task evaluates in its own language, `shell::read_sources` reads them in parallel into `Sources`, and `scan` takes the plan and those sources. The engine copies each text once per execution into the `String` that GritQL's `RichFile` takes. On the corpus that is 86 of 2,641 selected files, so the TypeScript no enabled rule evaluates is never read. A planned file missing from `Sources` fails the run as `SourceUnread`. One refusal moves: PR1 read every scannable file and refused the run on any file it could not read as UTF-8, and PR3 refuses only when the unreadable file is one a task evaluates. Findings are unchanged for every input either version accepts. Language keys are GritQL's `language_name()`, not the `Debug` dump of the grammar's node table that PR1 used: that string is 652 bytes for JSON and cost about 40 µs per file, 0.2 s of the walk and selection on the corpus. Governs R15.
- KTD19. **The benchmark is a flake check on `x86_64-linux`.** `crates/gritlint_bench` is a workspace binary over `gritlint_core` and rayon. It runs the baseline and the candidate over the corpus with `crates/gritlint_bench/effect.gritlint.json`, interleaving them for N rounds so both see the same machine. It fails when stdout or the exit code differs between the baseline, the candidate, and the candidate under `RAYON_NUM_THREADS=1` (PR4 narrows the baseline comparison, KTD26), and when the baseline exits with anything but 1, because a broken corpus or config, or one with no findings, would make every report equal. It then runs the candidate's pipeline in process on a one-thread pool for N rounds, timing config decode, rule compile, file walk, plan, file read, execution and report output, and fails when that report differs from the baseline's. Execution splits into parse, timed by parsing each file an execution reads with `engine::parse`, and match, the scan's wall time minus parse; a multifile rule loads only the files its pattern names, so parse is an upper bound. Scan and census build each execution with the same `decide::executions` constructor. The run fails when a parse yields no tree, when the census's files differ from `sources_needed`, or when parse exceeds the scan's wall time. It writes the median wall times, their ratio, N, the core count, the per-phase medians and shares, and the file, rule, execution and parse counts to `$out/summary.md`. The bench builds with the release profile over `gritlint-unwrapped`'s release dependency artifacts, so the phases time the shipped code generation. `checks.x86_64-linux.gritlint-bench` runs it, with the corpus from `fetchFromGitHub` and the other `x86_64-linux` checks as inputs, so it times on an otherwise idle builder. The `x86_64-linux` Nix job appends `summary.md` to the step summary. When the inputs are unchanged, the cached report for that exact binary pair is shown. Governs R16, R17.
- KTD20. **PR3 tests what is deterministic and leaves the thread schedule to the benchmark (KTD8).** The benchmark's differential is the equivalence test. Three `scan_contract` cases pin the behaviour PR3 changes, none of which depends on the schedule: the refusal names the first unparseable file in plan order, which a reversed fold would get wrong; an unreadable file refuses the run only when a rule evaluates it (KTD18's moved refusal); and a planned file missing from `Sources` refuses with `SourceUnread` instead of being evaluated as empty, which would be a false clean pass. A test of thread-schedule independence would fail under a broken implementation only on some schedules, so it would be flaky rather than a regression gate, and the admission gate refuses it. Governs R15.
- KTD21. **Rules declare `reasonCode`, `nextAction` and an optional `locate` in frontmatter.** A `---` block at the top of a `.md` or `.grit` rule holds one `key: value` per line. `reasonCode` matches `^[a-z][a-z0-9]*(-[a-z0-9]+)*$`; `nextAction` is one imperative sentence; `locate` is a GritQL pattern (KTD22). A missing required key, an unknown or repeated key, or a malformed `reasonCode` refuses the load, naming the file and the key. The block's lines are blanked, not removed, before compilation, so GritQL's positions stay the file's own. `parse_markdown` already skips a frontmatter block. Three single-line keys do not justify a YAML crate, and `Cargo.lock` has none. Requiring the keys breaks no known consumer: the monorepo's `gritlint.json` at `df908ddf` declares packs only, and this repository's declares no `rules`. A consumer's own rule file, listed under `rules` or passed with `--rules`, is held to the same keys: the run refuses, naming the file and the missing key, and `npm/gritlint/README.md` documents the frontmatter. Rejected: defaulting a consumer rule's keys, which would make `reasonCode` and `nextAction` nullable for every consumer of the envelope. All 15 pack rules gain the keys; `cell-architecture/service-exports-no-layer`'s inline "Next:" sentence moves into `nextAction`. Governs R18.
- KTD22. **Locations come from engine ranges and rule locators.** For a single-file rule, each range in an engine match is one finding at that range, and a match with no range is one finding at its file alone; PR3 kept only the first range. For a multifile rule, the rule's `locate` pattern is compiled at load as a single-file pattern in the rule's language, with the rule's pack library and parameters. After the scan it runs over the directory's matched files in path order, and the finding goes to the first file where it matches, at its first range in document order. Each locator reports the innermost violating node that exists: the wrong value's pair, else the enclosing key the violation lives in (for `npm-provenance/repository-url`, the `url` inside `repository`, else `repository`; never `bugs.url`). Appendix D lists every bundled rule's target. When no file matches, the violation is an absent key with no node, and the finding stays on the first matched path, as in PR3, with the file alone as its location. Every bundled rule declares `locate`. Each bundled bad fixture case commits its expected location in `expected-locations.txt`, and `gritlint test packs` fails a case whose reported locations differ, or a rule with `locate` none of whose bad cases expects a range. The three line-1 sites in `decide.rs` (`first_match` twice, `finding_for`) are deleted. Rejected: failing the run on a match with no range, which would refuse every bundled rule; reading a multifile variable's range, which can point into another file of the same execution (Dependencies / Assumptions); and no locators (Q4). Governs R18.
- KTD23. **One versioned envelope, its schema committed.** `--format json` emits `schemaVersion: 1`, `findings` (`ruleId`, `reasonCode`, `message`, `nextAction`, `location`: `path` and `range`, where `range` holds `start` and `end`, each a `line` and `column` as GritQL reports them, or is `null`), and `summary` (`filesSelected`, `findings`, `rules` with `ruleId` and `files`, `zeroFileRules`). Keys are camelCase, so PR1's `files_selected` is renamed; the version starts here, and a later incompatible change raises it. The serde types derive `JsonSchema`; `gritlint schema --report` prints the report schema, committed as `npm/gritlint/report_schema.json`, and `checks.<system>.gritlint-schema` diffs both schemas. Exit codes stay 0, 1 and 2. Governs R18.
- KTD24. **The concise reporter has a stated budget.** `--format concise` prints at most 20 finding lines of at most 200 bytes each (`path[:line:column] ruleId reasonCode message`; the message is cut first, then the start of the path, each on a character boundary with `…`, so the file name survives), then `… and N more; --format json lists every finding` when more exist, then the verdict line: at most 22 lines and 4,500 bytes. The constants live in `encode.rs`, and `--help` states the budget. `human` stays the default and stays unbounded. Governs R19.
- KTD25. **Two GitHub encoders, because the sandbox cannot write the summary.** The sandboxed binary runs under `--clearenv` with only the working directory bound, read-only, so it sees neither `GITHUB_STEP_SUMMARY` nor a writable path. `--format github` prints one `::error` command per finding: `file`, plus `line`, `col`, `endLine` and `endColumn` when the location has a range, and `title` from the rule id and reason code; the data is the message and the next action. Data escapes `%`, `\r` and `\n`; property values also escape `:` and `,` (actions/toolkit `escapeData` and `escapeProperty`). `--format github-summary` prints Markdown (verdict, counts, a table of at most 100 findings and the remainder count), which the CI step appends to `$GITHUB_STEP_SUMMARY`. GitHub renders at most 10 error annotations per step and 50 per job, so the summary is where a long list is read. Both are pure encoders over `Outcome`. Governs R19.
- KTD26. **Tests by layer.** `skill://test-layer-selection` admitted, default refuse: (a) in-process `scan_contract` cases for KTD22's contract; (b) one in-process `rule_loading` case table for KTD21's refusals; (c) properties in `tests/laws.rs`, where `encode` already has laws, for KTD24's budget and KTD25's escaping, because encoders are pure; (d) one e2e journey for the seam only a process boundary shows, the envelope and exit code of the sandboxed binary. The journey is `apps/gritlint-e2e`, a std and `serde_json` binary run by `checks.<system>.gritlint-agent-journey` against `packages.<system>.gritlint` inside the build, as `gritlint-is-sandboxed` runs the wrapper. A shell consumer is out because no approved executable parses JSON (R11, KTD9), and an integration test may not spawn the CLI. Refused: a dedicated `.grit`-frontmatter compile test (every existing `.grit` test rule gains frontmatter, so a leaked block fails them all) and per-format output snapshots (PR5 owns snapshots). The benchmark's baseline-to-candidate check compares the exit code and the count of findings per rule id parsed from human output, because PR4 changes human lines, moves `cell-architecture/service-exports-no-layer`'s "Next:" sentence out of its message (that pack is enabled in `crates/gritlint_bench/effect.gritlint.json`), and may move a finding to the file its locator matches. Candidate against candidate on one thread stays byte-identical, and the locator run is timed as its own phase. cargo-mutants arrives in PR5; its first run reports these properties' survivors. Governs R18, R19.
- KTD27. **Bundled cases also run in-process.** cargo-mutants counts a mutant caught only when `cargo test` fails, and today only `checks.<system>.gritlint-packs` runs the bundled cases, through the binary (`nix/gritlint-checks.nix:59-63`). A `gritlint_core` integration test, `tests/bundled_packs.rs`, reads `packs/` as `apps/gritlint`'s `test` subcommand does (`shell::read_tree`, `decode::fixture_layout`, `decide::run_fixtures`; `apps/gritlint/src/main.rs:166-171`), spawns no process, and fails listing every failing case. `packs/` is already in the crane source (`nix/gritlint.nix:67`), so `gritlint-nextest` runs it. `gritlint-packs` stays: it is the only run of the `test` subcommand and its exit code, and it costs seconds. Governs R28.
- KTD28. **The cases come from four planning gap analyses (Appendix E).** For each clause of each rule and each R20 defect, an analysis named the existing case that fails or proposed the smallest case that would: 69 proposals, of which 67 are committed and 2 wait on Q6. Expected locations were computed by counting characters in the proposed files, with the end column exclusive as in the committed lines (`npm-provenance/fixtures/public-access/bad/restricted` expects `10:5-10:27` for the 22-character pair at column 5). ce-work runs `gritlint test packs` on the unmodified rules. If a case disagrees, the count or the analysis is re-checked against the rule. The expectation is never edited just to make the case pass. The analyses, with every proposed case's files and expected lines, are committed beside this plan in `docs/plans/2026-10-10-1309-pr5-rule-cases/`. They are planning evidence: each is checked by ce-work's run, not trusted. Governs R20.
- KTD29. **Cases that pin nothing are deleted; dead clauses are removed only where proven.** `source-resolution/internals-export-map` `good/internals-dist-only` is deleted when `good/internals-other-condition` lands. No R20 defect makes it report, because its subpath has two keys and every pattern in the rule needs three. `cell-architecture` `bad/live-alias` stays: it fails under the same defects that fail `bad/layer-beside-clean-port`, so it is redundant but can fail. `shared-vitest-config`'s `locate` conjunct `$list <: not contains condition()` is deleted. The list is inside the program, so whenever the list contains the condition the program does too, and the next conjunct, `$program <: not contains condition()`, already fails whenever the first would; no input distinguishes the rule with or without it. The other conjuncts no case can pin stay (Appendix E): whether they are dead depends on whether `contains` over JSON matches text or structure, which U8's compact-JSON check settles before any is removed. Governs R20.
- KTD30. **The mutants workflow (pending Q8).** `.github/workflows/mutants.yml` runs on `push` to `main` with `concurrency` cancelling an older `main` run, as one `x86_64-linux` job. cargo-mutants 27.1.0 (MIT) comes from the locked nixpkgs, `nix shell --inputs-from . nixpkgs#cargo-mutants`, inside the workflow only and never in the dev shell. Cargo builds offline from `CARGO_HOME` holding `packages.<system>.gritlint-unwrapped.vendor`'s `config.toml`, as `ci.yml`'s `cargo deny` step sets it up; that vendor tree carries the engine's `resources/node-types` (`nix/gritlint.nix:81-87`), so a build needs no network. `.cargo/mutants.toml` limits mutation to `crates/gritlint_core/src`. `apps/gritlint` has no test that `cargo test` can run against it, since an integration test may not spawn the CLI (KTD26), so its mutants could only be missed; its process-boundary behaviour belongs to the journeys. The bench and e2e crates are measuring tools, not product. The job writes the count from `cargo mutants --list` to the step summary, runs `cargo mutants`, and on any exit uploads `mutants.out` and writes the outcome counts and the missed list to the step summary. It is not sharded. If the first run outlasts the job's time limit, a follow-up shards it as mutants.rs's Sharding page describes: every shard takes the same `n`, and the per-shard count comes from the measured time per mutant. Deferred to implementation: whether the job starts from crane's dev-profile dependency artifacts (`devArtifacts`, `passthru` on `gritlint-unwrapped`) or builds cold, decided by the first run's timings. Governs R21.
- KTD31. **PR5 adds no PR job.** The cases and `bundled_packs` run inside `ci.yml`'s existing per-system jobs, which took 7m45s (`x86_64-linux`) and 4m48s (`aarch64-linux`) on PR4 (run 38047097913). The mutants workflow never runs on a pull request. Governs R27.

### High-Level Technical Design

```mermaid
flowchart TB
  subgraph CI["PR1 CI (ci.yml)"]
    R["rust: nix develop -> gate-rust.sh<br/>fmt, clippy, loop check, tests,<br/>test packs, schema drift, features, deny"]
    N["nix x86_64-linux and aarch64-linux:<br/>windows refusal eval -> build every output<br/>-> gate-journey.sh -> ./bin/gritlint check"]
  end
  F["flake.nix"] -->|"packages.gritlint = sandbox(unwrapped)"| N
  F -->|"checks.gritlint-is-sandboxed, gritlint-unwrapped-is-plain"| N
  F -->|devShells.default| R
```

### Risks

| Risk | Consequence | Handling |
| --- | --- | --- |
| bubblewrap cannot create user namespaces on `ubuntu-24.04` runners | the journey fails | the monorepo's AppArmor sysctl step carries over |
| A consumer evaluates the `x86_64-windows` attributes | evaluation throws by design (R6) | `nix flake check` evaluates the current system only; AE2 runs the refusal eval explicitly, locally in U3 and in CI |
| The cold cargo build of the GritQL crates | slow first CI runs | PR2's crane dependency derivations and store cache (KTD10 to KTD13) |
| A dependency needs a newer compiler than `rust-version` | `gritlint-msrv` fails | raise `rust-version` to the measured minimum, which keeps the declared MSRV true |
| The store-cache bundle outgrows its savings | warm runs spend more on restore than they save | measured in R14; the export roots exclude everything cache.nixos.org signs, though the closures of the roots carry some signed paths |
| The benchmark shares a runner with other builds | noisy wall times | the bench check takes the other `x86_64-linux` checks as inputs, interleaves runs and reports medians and a ratio |
| R17's 10x hits a floor | PR1 takes 0.86 s to 0.97 s on the corpus on an 8-core host. Compiling all 15 rules costs 0.04 s, and reading every scannable file about 0.1 s single-threaded | the PR states the measured ratio and the floor that bounds it. Measured in PR3 by the bench's one-thread phase table: GritQL's match is about 90% of the wall time and parse at most 5%; no other phase reaches 3%. Each of the 86 read files is parsed once per execution that loads it, up to 8 times, so parsing each file once would save at most the parse share less one parse per file |
| The PR1 baseline brings its own vendor tree and release build into the store cache | the `x86_64-linux` bundle grows | measured in PR3; the vendor is a fixed-output derivation from the same `Cargo.lock`, so a shared output path adds nothing, and the PR body states the bundle size either way |
| A rule's locator matches a node other than the violating one | the location is exact but names the wrong node, and an agent rewrites a valid field | each locator reports the innermost violating node (KTD22, Appendix D); every bundled bad fixture case commits its expected location and `gritlint test packs` fails on a difference; every bundled rule has at least one bad case whose expected location carries a range, so a line is asserted |
| Single-file findings split per range | a consumer counting findings sees more than PR3 printed | no bundled rule is single-file; the envelope's version starts at 1 in this PR |
| A path longer than the concise line budget | that line loses the start of its path | the message is cut first, then the path's start, keeping the file name; `--format json` carries every path whole |
| The mutants workflow first runs after merge | a workflow error shows on `main`, not on the PR, since the PR trigger is forbidden and this account's token cannot dispatch | the job is plain shell around cargo-mutants; with Q7(b) its `--list` command runs locally before merge; otherwise the follow-up that cites the first report fixes it |
| A full mutants run outlasts the job's time limit | the run is cut off before its summary | the summary's first line is the `--list` count and `mutants.out` uploads on any exit; a follow-up shards the job from the measured time per mutant (KTD30) |
| A proposed case's expected location is miscounted | `gritlint test packs` fails in ce-work | re-check the count and the analysis against the rule (KTD28); never edit the expectation just to make the case pass |
| Q5(a) tightens `vitest-conditions` | a consumer whose config sets only the ssr list gets a new finding | a changeset intent states the change; the finding's `nextAction` already says to set both |

---

## Implementation Units

### U1. Verbatim import (PR1)

- **Goal:** the Appendix A files, byte-identical to `df908ddf`.
- **Requirements:** R1 to R3.
- **Dependencies:** none.
- **Files:** Appendix A.
- **Approach:** `git archive df908ddf <paths>` extracted into the work tree of `feat/import-gritlint`, which is cut from `main` at `1682ab5`. The monorepo commit is never checked out, merged or used as a base.
- **Test scenarios:** Test expectation: none -- a file copy; AE3 is the proof.
- **Verification:** AE3 over all 191 files; `git merge-base HEAD df908ddf` finds no common ancestor.

### U2. Identity and licence (PR1)

- **Goal:** imported metadata names this repository, and every licence surface is MIT under "System F Software" with the GritQL notices.
- **Requirements:** R4, R9.
- **Dependencies:** U1.
- **Files:** Appendix B, root `LICENSE`.
- **Approach:** point `[workspace.package] repository`, `nix/gritlint.nix` `meta.homepage` and `gritlint.json`'s `repositoryUrl` at this repo. Drop the `repos/**` ignore, which names a monorepo directory, and reword the "whole monorepo" comment. Point `bin/gritlint`'s no-nix message at the Nix installer instead of a CONTRIBUTING.md this repo does not have. Change `npm/gritlint/LICENSE`'s first holder, and add the two GritQL copyright lines to the root `LICENSE`.
- **Test scenarios:**
  - AE5: `./bin/gritlint check` exits 0 at the root.
  - `cargo metadata --no-deps` reports this repository for both crates.
- **Verification:** `git diff df908ddf HEAD -- <Appendix B paths>` shows only those edits.

### U3. Flake (PR1)

- **Goal:** the flake outputs consumers use today, served from this repository.
- **Requirements:** R5 to R7; KTD1, KTD2, KTD5.
- **Dependencies:** U1.
- **Files:** `flake.nix`, `flake.lock`, `.gitignore`.
- **Approach:** KTD1 and KTD2, locked to the KTD1 revisions. `.gitignore` carries `/target/`, `/result`, `/result-*` and `.direnv/`.
- **Test scenarios:**
  - `nix build .#packages.x86_64-linux.gritlint` succeeds and `result/bin/gritlint --version` prints `0.1.0`.
  - AE2 holds.
  - AE4: each break it lists, made in a scratch copy, fails the matching check; the copies are deleted.
  - `nix flake check` passes.
  - AE1 after the branch is pushed: a scratch consumer flake outside the repo builds `packages.x86_64-linux.gritlint` through `github:systemfsoftware/gritlint/feat/import-gritlint`. The result goes in the PR body.
- **Verification:** the scenarios above, run locally.

### U4. CI (PR1)

- **Goal:** the PR head shows the Rust gate, the Nix build of every output on both systems, and the dogfood journey.
- **Requirements:** R8 to R11; KTD3, KTD4.
- **Dependencies:** U2, U3.
- **Files:** `.github/workflows/ci.yml`, `scripts/tools/gate-journey.sh`.
- **Approach:** KTD3. Triggers are every pull request, whatever its base, so stacked PRs run CI, plus pushes to `main` and `workflow_dispatch`, with `permissions: contents: read`; concurrency cancels superseded PR runs.
- **Test scenarios:**
  - Locally, `nix develop --command sh scripts/tools/gate-rust.sh` exits 0.
  - Locally, `sh scripts/tools/gate-journey.sh <sandboxed gritlint>` exits 0, where this host permits user namespaces.
  - The journey exits 1 naming the case when a planted violation does not exit 1 or does not name its rule.
- **Verification:** the `rust` job and both `nix` jobs green on the PR head; job durations recorded as PR2's baseline.

### U5. Crane, nextest, MSRV and CI cost (PR2)

- **Goal:** cached Rust builds and a cheaper CI, measured.
- **Requirements:** R12 to R14.
- **Approach:** KTD10 to KTD14. `nix/gritlint.nix` returns the crane package with its dependency derivations and vendor tree in `passthru`. `flake.nix` adds the `crane` input and the KTD12 checks. `ci.yml` drops the `rust` job, replaces the closure cache with the KTD13 store cache, and runs `cargo deny check` on `x86_64-linux`.
- **Test scenarios:**
  - A source-only edit changes the gritlint derivation but not either dependency derivation's `drvPath`.
  - A library API stabilised after 1.90 (`Duration::from_mins`) in a scratch copy fails `gritlint-msrv`. `rust-version` is 1.90 because `tree-sitter-language` 0.1.8 requires it: with `rust-version = "1.89"`, `gritlint-msrv` fails naming that crate.
  - Each KTD12 check goes red on a planted break in a scratch copy: unformatted code, a clippy warning, a failing test, a broken pack fixture, an edited schema file, a `for` loop in `decide.rs`.
  - `gritlint-is-sandboxed`, `gritlint-unwrapped-is-plain`, AE2 and the journey still pass on the crane build.
  - The cold run and its re-run attempt are measured; the warm attempt's job minutes are below PR1's warm attempt and the monorepo's lanes.
- **Verification:** the R14 table in the PR body; every job green on the PR head.

### U6. Compile once, one parallel pass, benchmark (PR3)

- **Goal:** R15 to R17.
- **Approach:** KTD16 to KTD20. Measured on PR1 before planning: the 122 config files alone take 0.62 s of the 0.86 s, because every multifile rule recompiles for each directory, and the 2,500 TypeScript files that no enabled rule evaluates add 0.24 s, because their contents are cloned at each stage. Rayon is already in `Cargo.lock` through `marzano-core`, so making it a direct dependency adds no crate.
- **Test scenarios:**
  - On the corpus, the PR1 binary, the PR3 binary and the PR3 binary under `RAYON_NUM_THREADS=1` print byte-identical JSON and exit codes (differential, inside the benchmark).
  - In `scan_contract`, each of KTD20's three cases fails on the regression it names: a reversed fold, empty contents in place of `SourceUnread`, and reading every walked file as PR1 did.
  - `gritlint-bench` fails in a scratch copy where the candidate drops one finding.
  - The fixture suites, `gritlint test packs` and the dogfood journey pass unchanged.
- **Verification:** the CI step summary and job log with both median wall times, their ratio and the one-thread phase table; R17's ceiling stated in the PR from that table if the ratio is below 10.

### U7. Agent-first output (PR4)

- **Goal:** R18, R19.
- **Approach:** KTD21 to KTD26. Rule frontmatter supplies `reasonCode`, `nextAction` and the locator. `decide` builds findings with an optional range, and a post-scan step runs each multifile rule's locator over its directory's matched files. `encode` gains the envelope, `concise`, `github` and `github-summary`; the `apps/gritlint` `Format` enum and `schema --report` follow. CI's conventions step runs `--format github` and appends `--format github-summary` to the step summary.
- **Files:** `crates/gritlint_core/src/{decode,domain,decide,encode,engine}.rs`, `apps/gritlint/src/main.rs`, `packs/*/rules/*.md` (15), `packs/*/fixtures/*/bad/*/expected-locations.txt` (new, one per bad case: one line per finding, `ruleId path` or `ruleId path:startLine:startColumn-endLine:endColumn`, excluded from the case's selected files as `gritlint.json` is), three new bad cases (Appendix D), `npm/gritlint/report_schema.json` (new), `apps/gritlint-e2e/` (new), `Cargo.toml` and `Cargo.lock` (workspace member), `nix/gritlint-checks.nix`, `nix/gritlint.nix` (vendor hash re-checked), `nix/gritlint-bench.nix`, `crates/gritlint_bench/src/{main,breakdown}.rs`, `.github/workflows/ci.yml`, `npm/gritlint/README.md`, rule-format and fixture-format text in `packs/*/README.md`. Tests: `crates/gritlint_core/tests/{scan_contract,rule_loading,laws,fixture_runner}.rs`; existing `.grit` test rules, including `fixture_runner.rs`'s inline `NO_CONSOLE` and `CONDITION_RULE`, gain frontmatter.
- **Test scenarios** (each fails on the named regression):
  - `scan_contract`: a multifile rule whose locator matches only the second matched file reports that file at the locator's range. Fails if a line-1 fallback returns, if the locator runs only on the first path, or if a variable range is read.
  - `scan_contract`: a multifile rule whose locator matches nothing reports the first matched path with `range` null. Fails if any line is invented.
  - A single-file match that carries no range reports one finding at its file alone. Fails if the match is dropped, which would exit 0 on a violation. A `scan_contract` case if some single-file pattern yields a rangeless match; otherwise a `decide` unit test over an `EngineMatch` with no ranges.
  - `fixture_runner`: a bad case whose reported location differs from its `expected-locations.txt` fails, naming the rule and the case. Fails if a locator names another node, such as `bugs.url` for `npm-provenance/repository-url`.
  - `fixture_runner`: a pack rule with `locate` none of whose bad cases expects a range fails, naming the rule. Fails if a rule could ship with only file-alone cases, leaving its locator unasserted.
  - `gritlint test packs` over the bundled packs: every Appendix D case passes with its committed location, including the three new cases.
  - `scan_contract`: a single-file rule matching two nodes in one file yields two findings with distinct ranges. Fails if only the first range is kept.
  - `rule_loading`: missing `reasonCode`, missing `nextAction`, a repeated key, an unknown key (`location:`) and `reasonCode: Bad_Code` each refuse to load, naming the file and the key. Fails if a key defaults silently or a typo is ignored.
  - `laws`: for any outcome, concise output has at most 22 lines, 4,500 bytes and 200 bytes a line, is valid UTF-8, and its remainder count is the number of findings not shown.
  - `laws`: for any string, an escaped property value contains no `\r`, `\n`, `:` or `,`, an escaped data value no `\r` or `\n`, every `%` starts an escape, and unescaping returns the input. A file-alone finding's annotation has no `line` property.
  - Journey (`gritlint-agent-journey`): on a scratch tree with planted `npm-provenance/repository-url` and `typecheck-build-mode` violations, the consumer reads the sandboxed binary's JSON, requires `schemaVersion` 1, acts by `reasonCode` (rewriting exactly the reported range, or editing the file when the location is the file alone), and reruns until exit 0. A location off by a line or column corrupts the rewrite and the rerun fails.
- **Verification:** every `x86_64-linux` check builds locally, including `gritlint-schema`, `gritlint-agent-journey` and the bench, whose report shows the locate phase; `gate-journey.sh` and `./bin/gritlint check` pass on the sandboxed binary; CI green on both systems with the `github-summary` step summary.

### U8. Rule cases and in-process bundled run (PR5)

- **Goal:** R20, R28.
- **Requirements:** KTD27 to KTD29.
- **Approach:** the harness already refuses a rule without a good case, without a bad case, or with `locate` but no bad case asserting a range (`decide.rs` `fixture_results`, 493-521), and compares each bad case's findings with its `expected-locations.txt` (`location_mismatch`). U8 adds the Appendix E cases against the unmodified rules, deletes `good/internals-dist-only` (KTD29), and adds `bundled_packs` (KTD27). It changes no rule.
- **Files:** `packs/*/fixtures/**` (the Appendix E case directories; `packs/source-resolution/fixtures/internals-export-map/good/internals-dist-only/` deleted), `crates/gritlint_core/tests/bundled_packs.rs` (new).
- **Execution note:** one rule at a time: add its cases and require `gritlint test packs` green on the unmodified rule before moving on. If Q7(a) allows, red-probe each new case on a scratch copy under the Appendix E defect it names, delete the copy, and commit nothing from it. Before the npm-provenance and typecheck-build-mode cases, settle whether JSON `contains` matches text or structure: run the unmodified rules on a scratch copy of `provenance/good/provenance-true`, `public-access/good/public` and `typecheck-build-mode/good/tsc-build` rewritten as compact JSON. Clean verdicts mean `contains` is structural and Appendix E's `locate` value conjuncts are indistinguishable (R20's exception). A finding means the rule's verdict depends on formatting, a defect: commit the compact copy as a good case and fix the rule under the scope exception (U12).
- **Test scenarios:**
  - Every Appendix E case passes `gritlint test packs` on the unmodified rules, and each bad case reports exactly its committed lines. A location off by one column fails, naming the rule and the case.
  - `bundled_packs`: passes when every bundled case passes; when any case fails, it fails naming that case, so a mutant in `decode`, `decide`, `engine` or the locator that changes a bundled outcome is caught under `cargo test`.
  - Each R20 defect on each clause fails the case Appendix E names, or the clause is listed there as unpinnable, with the reason.
- **Verification:** `gritlint test packs` passes 139 cases (73 + 67 − 1); `checks.<system>.gritlint-nextest`, including `bundled_packs`, and `gritlint-packs` build on both systems.

### U12. Rule changes from the rulings (PR5)

- **Goal:** R20 for the clauses Q5, Q6 and Q9 decide, any formatting defect U8's compact-JSON check exposes, and the dead conjunct KTD29 removes.
- **Requirements:** KTD29; Q5, Q6 and Q9 answered.
- **Approach:** delete `shared-vitest-config`'s dead `locate` conjunct; existing cases stay green, since no input distinguishes the rule with or without it. Apply Q5: (a) makes `vitest-conditions` require both condition lists, adds the client-pair `locate` branch, and adds `bad/client-other-condition` and `bad/only-ssr-conditions`; (b) narrows the `nextAction` and README and adds a good case pinning the ssr-only reading. Apply Q6: commit GAP 7 and GAP 12 as good or bad cases, changing a rule only if a ruling makes the value a finding. Apply Q9 the same way. Apply any formatting defect U8's compact-JSON check exposes. A changeset intent states each change in what a rule reports; a ruling that pins today's behaviour changes nothing and needs none.
- **Files:** `packs/source-resolution/rules/shared-vitest-config.md`, and as the rulings require `packs/source-resolution/rules/{vitest-conditions,tsconfig-paths,tsdown-exports}.md`, `packs/npm-provenance/rules/*.md`, `packs/typecheck-build-mode/rules/typecheck-build-mode.md`, the packs' READMEs, their new case directories, `.changeset/*.md`.
- **Test scenarios:**
  - Every existing `shared-vitest-config` case passes after the conjunct is deleted, with unchanged expected locations.
  - With Q5(a): `vitest-conditions` `bad/client-other-condition` reports the client `conditions` pair, `bad/only-ssr-conditions` names the file alone, and `good/both-conditions` stays clean.
  - Each Q6 case passes as ruled; for a "finding" ruling, the case fails against the rule before its change.
- **Verification:** `gritlint test packs` passes 139 cases plus the ruling cases committed (two for Q5(a) or one for Q5(b), up to two for Q6, up to three for Q9, and one per formatting defect U8 exposes); a changeset intent exists for each change in what a rule reports.

### U11. Main-only mutants workflow (PR5)

- **Goal:** R21, R27.
- **Requirements:** KTD30, KTD31; Q7 and Q8 answered.
- **Approach:** `.github/workflows/mutants.yml` and `.cargo/mutants.toml` per KTD30. No PR job is added (KTD31). With Q7(b), the job's `cargo mutants --list` command runs locally before merge, which builds and runs no mutant.
- **Files:** `.github/workflows/mutants.yml` (new), `.cargo/mutants.toml` (new).
- **Test expectation:** none. A workflow is configuration, and it runs only on `main`; evidence is the first run.
- **Verification:** `ci.yml` stays green on the PR with every job at or under 10 minutes, read from the jobs API, and the PR head shows no `mutants` run; after merge, the first `mutants` run on `main` and its step summary, cited in a follow-up with the missed list.

### U9. Tarball, release and changeset check (PR6)

- **Goal:** R22 to R24.
- **Approach:** statically linked binaries per platform (Q2), a launcher `bin` that selects by platform and architecture, and the tarball as a flake output. The change that drops `private` from `npm/gritlint/package.json` also adds `repository.url` naming this repository and `publishConfig` with `access: "public"` and `provenance: true` (inert, since nothing publishes to a registry), so the `npm-provenance` pack keeps R9's dogfood gate green. `release.yml` is the `df908ddf` caller verbatim: the reusable `release.yml` at `8cd6e830` with `ci-workflow: ci.yml` and `contents: write`, `pull-requests: write`, `actions: write`. `changeset-check.yml` calls the sibling at `8cd6e830` in its default mode (`devshell: false`: `pnpm/action-setup`, `actions/setup-node`, `pnpm install --frozen-lockfile`). `release.jsonc` adapts the monorepo's. KTD2's check extends to every release output. The PR description carries a one-line note if the release workflow needs "Allow GitHub Actions to create and approve pull requests"; that is not a blocker.
- **Test scenarios:**
  - AE8 in CI.
  - An unsupported platform gets an error naming the supported ones.
  - A PR touching `npm/gritlint/` without an intent fails the changeset check.
- **Verification:** the consumer journey green on both systems; a local release dry run.

### U10. LSP and Biome export (PR7)

- **Goal:** R25, R26.
- **Approach:** `gritlint lsp` on `lsp-server` and `lsp-types`, reusing PR4's findings and PR3's compiled rules. `gritlint export biome` renders single-file rules into `.grit` plugins with `register_diagnostic` and refuses the rest with reason codes.
- **Test scenarios:**
  - Opening a file with a violation publishes one diagnostic at the engine range; fixing it publishes an empty set.
  - The exporter refuses a multifile rule with its reason code.
  - One exported-plugin snapshot per exportable rule; Q1 decides how they are validated.
- **Verification:** the in-process tests and Q1's validation.

```mermaid
flowchart LR
  U1 --> U2 --> U4
  U1 --> U3 --> U4
  U4 --> U5 --> U6 --> U7 --> U8 --> U12 --> U11 --> U9 --> U10
```

---

## Verification Contract

| Gate | Command or check | Proves |
| --- | --- | --- |
| Provenance | AE3 blob-hash and mode comparison | R1 |
| Ancestry | `git merge-base HEAD df908ddfe497e53c1c03b92519a96c913c441a58` exits 1 | R2 |
| Rust gate (PR1; PR2 replaces it with the PR2 gate row) | `nix develop --command sh scripts/tools/gate-rust.sh` | R3, R8 |
| Flake | `nix flake check`; AE2; the AE4 probes; `nix build` of each output | R5 to R7 |
| Licence | root `LICENSE` and `npm/gritlint/LICENSE` name "System F Software" and carry the Iuvo AI and Biome Developers notices; `Cargo.toml` declares `license = "MIT"` | R4 |
| Dogfood | `./bin/gritlint check`; `scripts/tools/gate-journey.sh` | R9 |
| Consumer switch | AE1 scratch consumer flake | R5 |
| CI | `ci.yml` green on the PR head on both systems, read through `run_watch` | R8 to R11 |
| PR2 gate | `nix build` of every `checks.<system>` attribute; `nix develop --command cargo deny check` | R12, R13 |
| PR2 cost | R14 table from `/timing` and the attempt jobs API | R14 |
| PR3 benchmark | `checks.x86_64-linux.gritlint-bench`, its `summary.md` in the `x86_64-linux` step summary | R15 to R17 |
| PR4 output | `checks.<system>.gritlint-schema`, `gritlint-nextest`, `gritlint-agent-journey`; the `github-summary` step summary | R18, R19 |
| PR5 cases and mutants | `gritlint test packs` (139 cases plus the ruling cases); `checks.<system>.gritlint-nextest` with `bundled_packs`; PR job times from the jobs API; no `mutants` run on the PR head; the first `main` `mutants` run's summary | R20, R21, R27, R28 |

## Definition of Done

- Every PR1 unit's verification holds, and AE1, AE3 and the AE4 probe are recorded in the PR body.
- One PR from `feat/import-gritlint` into `main`, a single-layer `gh stack`, CI green on its head.
- The PR body cites `df908ddfe497e53c1c03b92519a96c913c441a58`.
- No PR1 file outside Appendices A to C changes, and no scratch probe remains.
- No force-push, and no monorepo commit in any pushed ancestry.

---

## Appendix

### A. Imported files (at `df908ddf`; 191 files)

| Source path | Files |
| --- | --- |
| `apps/gritlint/` | 2 |
| `crates/gritlint_core/` (sources, tests, `tests/fixtures/engine/**`) | 28 |
| `packs/cell-architecture/**` | 16 |
| `packs/npm-provenance/**` | 33 |
| `packs/source-resolution/**` | 83 |
| `packs/typecheck-build-mode/**` | 14 |
| `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `clippy.toml`, `deny.toml` | 5 |
| `npm/gritlint/` (`package.json`, `README.md`, `LICENSE`, `configuration_schema.json`) | 4 |
| `nix/gritlint.nix`, `nix/gritlint-sandbox.nix` | 2 |
| `bin/gritlint` | 1 |
| `gritlint.json` | 1 |
| `scripts/tools/gate-rust.sh` | 1 |
| `docs/solutions/tooling-decisions/gritlint-conventions-platform.md` | 1 |

### B. Imported files changed in PR1

| Path | Change |
| --- | --- |
| `Cargo.toml` | `[workspace.package] repository` |
| `nix/gritlint.nix` | `meta.homepage`; the "whole monorepo" comment reworded |
| `gritlint.json` | `repositoryUrl`; `repos/**` ignore removed |
| `npm/gritlint/LICENSE` | first holder "System F Software" |
| `bin/gritlint` | no-nix message |
| `docs/solutions/tooling-decisions/gritlint-conventions-platform.md` | `module`, `related_components`, and the dprint link pointed at the monorepo at `df908ddf` |

Root `LICENSE`, the one pre-existing file, gains the GritQL notices.

### C. New files in PR1

| Path | Unit |
| --- | --- |
| `flake.nix`, `flake.lock`, `.gitignore` | U3 |
| `.github/workflows/ci.yml`, `scripts/tools/gate-journey.sh` | U4 |
| `docs/plans/2026-10-10-0332-feat-gritlint-standalone-plan.md` | PR1's issue of this plan |
| a `docs/solutions/` learning from ce-compound, if one is recorded | — |

### D. PR4 locator targets

Each locator reports the innermost violating node that exists. "File alone" lists the absent-key cases, where the engine has no node. No bundled rule's violation can only be an absence. The three new bad cases were confirmed as violations in planning: with them added to a scratch copy of `packs/`, `gritlint test packs` passed 73 cases on the current build.

| Rule | Node the locator reports | Bad cases asserting a line | File alone (absent key) |
| --- | --- | --- | --- |
| `cell-architecture/service-exports-no-layer` | the `Layer.*` call, `*Live` export, layer export statement or specifier, or `layer` field | all 14 existing cases | none |
| `npm-provenance/provenance` | the `provenance` pair in `publishConfig`, else the `publishConfig` pair | `provenance-false` | `no-publish-config` |
| `npm-provenance/public-access` | the `access` pair in `publishConfig`, else the `publishConfig` pair | `restricted` | `no-publish-config` |
| `npm-provenance/repository-url` | the `url` pair in `repository`, else the `repository` pair; never `bugs.url` or `funding.url` | `other-repository` (`url`), `no-url` (`repository`) | `missing-repository` |
| `source-resolution/api-extractor-tsconfig` | the `customConditions` pair in `tsconfig.api.json` | `keeps-condition` | `tsconfig.api.json` without `customConditions` (no case) |
| `source-resolution/export-map-order` | the misordered `exports` subpath entry | `types-before-condition` | none |
| `source-resolution/internals-export-map` | the `exports` subpath entry carrying the condition | `internals-carries-condition` | none |
| `source-resolution/publish-config-exports` | the bare-string or condition-carrying entry in `publishConfig.exports`, else the `publishConfig` pair when it has no `exports` | `bare-string`, `leaks-condition` | no `publishConfig` (no case) |
| `source-resolution/shared-vitest-config` | the first `conditions` array without the condition | `conditions-without-condition` (new) | `missing-ssr` |
| `source-resolution/tsconfig-condition` | the `customConditions` pair without the condition | `other-condition` (new) | `no-condition-no-preset` |
| `source-resolution/tsconfig-paths` | the `paths` pair: the offending key is the package's own name, read from `package.json`, and a single-file locator cannot bind another file's value, so the enclosing pair is the narrowest node it can report | `maps-own-name` | none |
| `source-resolution/tsconfig-preset-module-mode` | the `customConditions` pair (empty or without the condition), else the `module` pair when neither `module` nor `moduleResolution` names a valid mode | `cleared-condition` | `no-module-mode` |
| `source-resolution/tsdown-exports` | `clean: false`, the `devExports` pair, or the `exports` pair missing `customExports` | `clean-false`, `dev-exports-true`, `missing-custom-exports` | none |
| `source-resolution/vitest-conditions` | the `ssr` pair without the condition | `ssr-other-condition` (new) | `only-client-conditions` |
| `typecheck-build-mode/typecheck-build-mode` | the `typecheck` script pair | `tsc-no-emit` | `package.json` without `typecheck` (no case) |

### E. PR5 cases per rule

From four read-only gap analyses at `69f7b67a` (KTD28), committed in `docs/plans/2026-10-10-1309-pr5-rule-cases/` with each case's files and expected lines. "Existing" counts good / bad cases. "Gaps" counts the clause defects that no existing case fails. One case can catch several gaps, so a row can list fewer cases than gaps. Gaps with no case are covered by the list below the table. Each new case names the clause whose defect it catches.

| Rule | Existing (good / bad) | Gaps | New cases (clause caught) |
| --- | --- | --- | --- |
| `cell-architecture/service-exports-no-layer` | 8 / 14 | 12 | good `driver-named-service` (`.service.ts` escaped dot); bad `export-let-binding`, `export-var-binding` (`const\|let\|var`); bad `underscored-binding-name` (name classes); bad `export-bound-to-spaced-nested-member` (member chain); good `layer-builder-binding` (`*Layer` anchoring); bad `semicolon-export-binding` (`;?`); bad `multiline-annotated-binding` (`[\s\S]` in the annotation); bad `reexported-layered-name` (`*Layer` re-export arm, digit class); bad `type-reexport-before-violation` (type re-export locate branches); good `underscored-layer-field` (field name class); bad `private-layer-field` (the class-field arm: tree-sitter names a `private readonly` field `public_field_definition`, so the rule reports it) |
| `npm-provenance/provenance` | 2 / 2 | 4 | good `other-json` (`package\.json` escaped dot); bad `private-false` (`"private": true` literal); good `no-name-package` (`"name"` conjunct); bad `provenance-string` (`"provenance": true` literal); bad `provenance-outside-publish-config` (Q9(1): `$publish`, not `$program`, holds the `"provenance": true` test); bad `provenance-false-outside-publish-config` (`locate`'s `within "publishConfig"`) |
| `npm-provenance/public-access` | 3 / 2 | 3 | good `other-json`; bad `private-false`; good `no-name-package` (same clauses as `provenance`) |
| `npm-provenance/repository-url` | 2 / 3 | 4 | good `other-json`; bad `private-false`; good `no-name-package`; bad `url-case-mismatch` (exact URL match) |
| `source-resolution/api-extractor-tsconfig` | 1 / 1 | 5 | good `no-tsconfig-file-path` (the `tsconfigFilePath` conjunct; api-extractor then reads `<projectFolder>/tsconfig.json`, its documented default); good `api-tsconfig-elsewhere` (same-directory join); good `decoy-file-names` (both file names' escaped dots). `locate`'s `not []` has no case (listed below) |
| `source-resolution/export-map-order` | 1 / 1 | 10 | bad `default-before-types`, `types-not-dist`, `foreign-condition-first`, `types-without-default`, `condition-and-types`, `condition-and-default` (one per `or` branch B2 to B7); bad `locate-scope` (`locate` held to `exports`); good `misordered-map-untouched` (file and directory scope); good `escaped-file-names` (escaped dots) |
| `source-resolution/internals-export-map` | 2 / 1 | 3 | good `internals-other-condition` (`$cond <: condition()`); bad `decoy-bundler-name` (bundler file-name regex); good `decoy-manifest-name` (manifest name regex). `good/internals-dist-only` deleted (KTD29) |
| `source-resolution/publish-config-exports` | 1 / 2 | 8 | bad `no-publish-config` (the missing-`publishConfig` branch); bad `publish-config-without-exports` (the `exports` conjunct); bad `bare-string-dot` (the `"."` branch); bad `leaks-condition-subpath` (the subpath branch); bad `locate-scope` (`locate` held to `publishConfig`); good `publish-config-untouched-elsewhere` (three scope conjuncts); good `publish-config-other-condition` (`$cond <: condition()`) |
| `source-resolution/shared-vitest-config` | 1 / 2 | 4 | good `parameter-selects-shared-config` (the fixed path conjunct and the `sharedVitestConfigFile` parameter); good `parameter-selects-decoy-name` (`base\.js` escaped dot). The fourth gap is the dead `locate` conjunct, deleted (KTD29) |
| `source-resolution/tsconfig-condition` | 4 / 2 | 4 | good `nested-tsconfig-file` (`[^/]*` file-name class); good `empty-include` (non-empty `include`); good `extends-preset-array` (the `contains presets()` branch: an `extends` array naming a preset, since `contains` cannot match a prefix of a quoted string); bad `cleared-conditions` (empty `customConditions`, `locate` first branch) |
| `source-resolution/tsconfig-paths` | 1 / 1 | 3 | bad `build-config-maps-own-name` (`tsconfig*.json` names); good `tsconfig-in-other-directory` (same-directory join); Q6(1): bad `maps-own-subpath`, `maps-own-subpath-wildcard` (the package-name part of a `paths` key); good `name-prefix-not-self`, `other-package-wildcard` (a longer name or another package stays clean) |
| `source-resolution/tsconfig-preset-module-mode` | 2 / 2 | 3 | good `module-mode-alternatives` (every `module` and `moduleResolution` alternative); bad `preset-under-tsc-and-node` (each path shape `presetFiles` accepts); bad `other-condition` (the condition predicate, `locate` second branch); bad `node10-module` (an invalid mode, the `module` locate branch) |
| `source-resolution/tsdown-exports` | 1 / 3 | 5 | good `other-bundler-config-ignored` (file filter); bad `missing-dev-exports` (absent `devExports`, `locate`'s third branch); bad `dev-exports-other-condition` (`condition()` on a string value); good `clean-true` (`clean` literal); good `exports-from-identifier` (Q6(2): `exports` bound to an identifier stays silent, a README Known limit) |
| `source-resolution/vitest-conditions` | 2 / 2 | 3 | good `non-vitest-config-ignored` (file filter); bad `mts-config` (the `.mts` alternative); good `imports-shared-double-quoted` (the parameter's quote class); Q5: bad `only-ssr-conditions` (the client `resolve.conditions` requirement; the file alone), bad `client-other-condition` (the client pair `locate` branch), good `merges-shared-config` (the shared-config import satisfies both lists) |
| `typecheck-build-mode/typecheck-build-mode` | 3 / 1 | 2 | bad `no-typecheck-script` (the missing-script branch; the file alone, per Appendix D); good `files-non-empty` (the `"files": []` literal); bad `root-reference-only` (a repository-root `tsconfig.json`, which `(.*/)` never matched; fixed to `(.*/\|)`); bad `second-typecheck-key` (`locate`'s `not includes` conjuncts); Q9: good `empty-references`, `tsc-build-verbose`, `tsc-build-long-verbose` |

Clauses no case can pin, left unchanged unless KTD29 deletes them or a ruling decides them:

- `npm-provenance/provenance` `locate` `$value <: not true` and `public-access` `locate` `$value <: not "public"`: `locate` runs only on files the rule already flagged, and JSON `contains` is structural (U8's compact-JSON check: good cases stay clean when compacted), so these filter nothing.
- `source-resolution/api-extractor-tsconfig` `locate` `$value <: not []`: a flagged `tsconfig.api.json` contains no `"customConditions": []`, so every value `locate` sees is already non-empty.
- `source-resolution/tsconfig-paths`: the `$target` in `` `$mapped: $target` `` is inert, since every JSON key has a value; stripping the quotes on both compared sides keeps them equal, so no fixture sees it.
- `source-resolution/export-map-order` `bad/types-before-condition` fails if either of two branches is dropped, so it pins that some branch reports, not which; the new per-branch cases pin each one.
