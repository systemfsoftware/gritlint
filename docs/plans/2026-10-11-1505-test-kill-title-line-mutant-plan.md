---
title: Kill the title-line mutant in parse_markdown - Plan
type: test
date: 2026-10-11
topic: kill-title-line-mutant
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-brainstorm
execution: code
source_commit: systemfsoftware/gritlint@d96dc51c1
---

# Kill the title-line mutant in parse_markdown - Plan

## Goal Capsule

- **Objective:** the one mutant that main's full Mutants run 38130956835 (at `d96dc51c1`) reported as Missed is caught by a `gritlint_core` test: `crates/gritlint_core/src/decode.rs:740:33: replace && with || in parse_markdown`.
- **Means:** one behaviour test in `crates/gritlint_core/tests/rule_loading.rs`. No production change.
- **Product authority:** the root's ruling: a row that is still Missed gets a follow-up test PR, not a re-argument.
- **Proof:** after this PR merges, the root dispatches `gh workflow run mutants.yml -R systemfsoftware/gritlint --ref main -f mode=in-diff -f base=5c9c2ab21`. It re-tests every mutant on the source lines that #14 changed, 740:33 included. A per-push run on a tests-only change lists 0 mutants, so it proves nothing.

## Product Contract

### Problem Frame

#14's U8 rewrite turned a let-chain (`title.is_none() && let Some(heading) = ...`, unviable at `5c9c2ab21`) into `if title_line.is_none() && trimmed.starts_with("# ")`. Under `||`, that condition is true at line 0, because `title_line` starts as `None`, and then on every `# ` heading outside a fence. So `title_line` ends on the last such heading, or on line 0 when there is none. The original keeps the first heading. Line 0 is the blanked frontmatter, and the description is trimmed, so a rule with zero or one heading gives the same message under both versions. With two `# ` headings before the grit block, the original drops the first heading from the message and keeps the second. The mutant drops the second and keeps the first. No existing test has two headings.

### Requirements

- R1. A rule whose markdown has `# Title`, prose, `# Later heading`, prose and then the grit block, loaded through `decode::load_rules`, has a `message()` that leaves out `# Title` and keeps `# Later heading` with its prose.
- R2. The input is a literal rule file in a tempdir pack. The expected message is a hand-written literal. The test imports no constants from the crate.

### Scope Boundaries

- No production code, changeset or mutants config changes. Nothing is mutated locally; only `cargo mutants --list` runs, to confirm the mutant's identity.

## Implementation Units

### U1. Two-heading description test

- **Requirements:** R1, R2.
- **Files:** `crates/gritlint_core/tests/rule_loading.rs`.
- **Test scenario:** a pack `pack-one` with `rules/two-headings.md`: frontmatter, `# Title`, `Prose under the title.`, `# Later heading`, `Prose under the later heading.`, then a `grit` block. `load_rules` with the pack enabled. Expected `message()`: `"Prose under the title.\n\n# Later heading\n\nProse under the later heading."`. The mutant would yield `"# Title\n\nProse under the title.\n\n\nProse under the later heading."`, which does not equal it. The test therefore passes on the original and fails on the mutant.
- **Verification:** `cargo test -p gritlint_core --test rule_loading` passes. `cargo mutants --list` still names `decode.rs:740:33`.
