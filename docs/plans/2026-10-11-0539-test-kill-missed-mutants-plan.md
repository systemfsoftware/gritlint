---
title: Kill the 72 missed mutants of the first full run - Plan
type: test
date: 2026-10-11
topic: kill-missed-mutants
artifact_contract: ce-unified-plan/v1
product_contract_source: ce-plan-bootstrap
execution: code
source_commit: systemfsoftware/gritlint@5c9c2ab21
---

# Kill the 72 missed mutants of the first full run - Plan

ce-doc-review ran (4 reviewers); amendments are sub-conductor rulings (plan gate, 2026-10-11: Q1-Q5 and audit items A1-A4, under Rulings).

## Goal Capsule

- **Objective:** every one of the 72 mutants that full run 38092194120 (`mode=full`, at `1b0437c7a`) reported as Missed is resolved one of three ways. 63 are caught by a `gritlint_core` test that drives real input through a public entry point and asserts an outcome a consumer can see. 7 are deleted with the code nothing reads (Q4 names nine rows: M-domain-03 gains a consumer and a test, and M-domain-01 stays because `NonEmpty::len` has a reader). Two are excluded by anchored `exclude_re` lines: M-engine-02 as equivalent (Q3), and M-domain-01, whose `is_empty` clippy requires beside a read `len` (Q4). Q4's decision rule can move a deleted row into the tested group. The 5 Timeouts stay Timeouts, which count as detected (Q5).
- **Means:** U0 re-pins the compound packs and enrols `mutation-discipline` (A1). U1-U7 are test files under `crates/gritlint_core/tests/`. U8 is one production commit (`refactor(core): ...`) with a patch changeset: it deletes the unread accessors and carries the refusal reason into `InvalidParamName`. U9 adds the `exclude_re` to `.cargo/mutants.toml`. Units are file-disjoint. No new dependency. Line numbers hold at `5c9c2ab21`: `git diff --stat 1b0437c7a 5c9c2ab21 -- crates apps Cargo.toml Cargo.lock .cargo` is empty. U8 shifts `domain.rs` lines, so the table's line:col is regenerated from `cargo mutants --list` after it.
- **Proof:** the post-merge per-push Mutants run on `main`, with no manual dispatch. It is a full run because this PR changes `.cargo/mutants.toml`, a full-run trigger against the last green `main` SHA (`scripts/tools/mutants-plan.sh:6,66-72`). It must report Caught for every Missed row the table assigns a test, list neither M-engine-02 nor the deleted accessors' mutants, and hold exactly the five Timeout rows in its timeout list.

## Product Contract

### Summary

Run 38092194120 tested all 475 mutants: 262 caught, 72 missed, 5 timeout and 136 unviable. Its survivor list was read from the run's `mutants.out-*` artifacts (`missed.txt`, `timeout.txt`, and the per-mutant `diff/` and `log/` files). Every survivor was then analysed against the code at `5c9c2ab21`, and its kill was argued from the mutant's diff alone, with no local mutation. The 72 Missed mutants resolve as follows. 63 get a test. Q4 names nine rows on code nothing reads: 7 are deleted, M-domain-03 gains a consumer (U8) and a test (U1), and M-domain-01 is excluded because `NonEmpty::len` has a reader (U8 decision rule). 1 (M-engine-02) is equivalent on every reachable input and is excluded (Q3). The 5 Timeouts stay Timeouts (Q5).

### Problem Frame

These mutants survived for four reasons:

1. **The only consumer is outside the mutated package.** cargo-mutants runs only `gritlint_core`'s own tests (`--package`, cargo-mutants `book/src/workspaces.md:39-41`). The contracts below are checked today only by the `apps/gritlint` CLI, by the `crates/gritlint_bench` crate, or by Nix flake checks (`nix/gritlint-checks.nix:72-82`, schema drift). Affected: GritlintConfig::schema/schema_for_packs, report_schema, embedded_packs, executions, the ScanPlan and Execution accessors, `shell::is_file` and `engine::parse`.
2. **The concise and summary renderers are covered only by property laws** (`tests/laws.rs`). The laws bound the line length but do not pin which bytes are kept, so 20 encode mutants stay inside them.
3. **No test exercises the boundary.** Examples: a heading inside an example fence, two `grit` blocks, a non-rule file in `rules/`, `node_modules` inside a fixture case, a rule calling a pattern from another scope, and a frontmatter refusal whose wording is not asserted.
4. **There is no consumer at all.** Nine accessors and renderings have no caller in the workspace. Under Q4 they are deleted, not tested.

### Requirements

- R1. Every row whose Run outcome is Missed and whose last column names a test is caught by that test. The argument is the mutant's diff applied to the stated input, without running the mutant. Each test asserts an outcome a consumer sees: rendered output, a returned plan/execution/pack value, a refusal message, or a CLI-visible decision. It never asserts calls, never restates the implementation, and never only re-reads a fixture. A Timeout row's disposition is stated in the Timeouts section, and the tests it names only make that Timeout deterministic. No test pins an accessor nothing reads (Q4).
- R2. Tests use real input: tempdir trees, the committed `packs/` tree, and the committed `npm/gritlint/*.json` contracts. There are no mocks and no new fixture directories. Expected values come from the published contract, never from the code under test (A2, A3).
- R3. Tests follow the current layout: one file per contract under `crates/gritlint_core/tests/`. Two files are new, `config_decode.rs` and `encode_contract.rs`, because no existing contract file owns config decoding or rendering (the other files are the ones U2, U4, U5, U6 and U7 extend). The lint header is the existing `#![allow(...)]` block.
- R4. Every PR job stays at or under 10 minutes, measured from the jobs API on the PR head (`Nix · x86_64-linux`, `Nix · aarch64-linux`). Every added test is an in-process run over a tiny tree or a string. The one larger input is the `packs/` tree, which `bundled_packs.rs` already reads.
- R5. A Timeout is never hidden. All five stay Timeouts, counted as detected, and none is excluded (Q5).
- R6. Code nothing reads at runtime is deleted, not tested (Q4; `mutation-discipline/unkillable-survivors-are-dead-code.md`, declared from U0). Decision rule: if a deletion fails to compile, the error names the reader; that reader is the consumer, and that row gets a test through it instead. At `5c9c2ab21` no test reads `ParsedRule::title()` or `ParsedRule::params()`. A refusal reason nobody prints gets a consumer: `GritlintError::InvalidParamName` carries it, and a patch changeset records the message change.
- R7. The equivalent mutant M-engine-02 is excluded by exactly one anchored `exclude_re` line with a comment stating why and when to remove it (Q3). M-domain-01 gets its own anchored line (Q4, U8 decision rule). `cargo mutants --list` before and after U9 differs by exactly those two names.
- R8. The proof is the post-merge per-push run on `main` (Q1).

### Mutant table

`ID` is the analysis id. Mutant names are cargo-mutants' names relative to `crates/gritlint_core/src/`, with line:col regenerated after U8 from `cargo mutants --workspace --list` at `a4589f4f1` (list only). The seven rows U8 deleted keep their `5c9c2ab21` name; their mutants are absent from that listing. Test names are the ones the units give.

| ID | Run outcome | Mutant (file:line:col: description) | Disposition |
| --- | --- | --- | --- |
| M-decide-01 | Missed | `decide.rs:174:50: replace && with \|\| in place` | U6 `a_multifile_locator_sees_only_the_files_the_rule_matched` |
| M-decide-02 | Missed | `decide.rs:186:39: replace match guard best.0 <= candidate.0 with false in place` | U6 `a_multifile_locator_tie_goes_to_the_first_path_not_the_engine_order` |
| M-decide-03 | Missed | `decide.rs:206:5: replace executions -> Result<Vec<Execution>, GritlintError> with Ok(vec![])` | U6 `an_execution_names_its_language_and_reads_only_its_own_language` |
| M-decode-01 | Missed | `decode.rs:53:9: replace GritlintConfig::schema -> Schema with Default::default()` | U1 `the_configuration_schema_is_the_committed_schema` |
| M-decode-02 | Missed | `decode.rs:58:9: replace GritlintConfig::schema_for_packs -> Schema with Default::default()` | U1 `the_configuration_schema_is_the_committed_schema` |
| M-decode-03 | Missed | `decode.rs:105:5: replace enabled_packs -> Result<BTreeMap<PackName, BTreeMap<ParamName, String>>, GritlintError> with Ok(BTreeMap::new())` | U1 `a_config_pack_entry_becomes_a_typed_pack_name_and_parameters`; U1 `a_pack_parameter_that_is_not_an_identifier_refuses_naming_the_key_the_config_and_the_reason` |
| M-decode-04 | Missed | `decode.rs:145:5: replace embedded_packs -> Result<Vec<Pack>, GritlintError> with Ok(vec![])` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-decode-05 | Missed | `decode.rs:161:42: replace == with != in embedded_packs` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-decode-06 | Missed | `decode.rs:171:54: replace && with \|\| in embedded_packs` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-decode-07 | Missed | `decode.rs:354:49: replace == with != in load_rules` | U5 `a_consumer_directory_with_two_rules_loads_both` |
| M-decode-08 | Missed | `decode.rs:412:44: replace && with \|\| in rule_sources_in` | U5 `a_pack_ignores_a_non_rule_file_in_its_rules_directory` |
| M-decode-09 | Missed | `decode.rs:442:5: replace is_scannable -> bool with true` | U6 `a_walk_returns_only_files_with_a_language_extension` |
| M-decode-10 | Missed | `decode.rs:447:5: replace is_rule_file -> bool with true` | U5 `a_pack_ignores_a_non_rule_file_in_its_rules_directory` |
| M-decode-11 | Missed | `decode.rs:501:44: replace && with \|\| in consumer_layout` | U4 `a_consumer_layout_ignores_a_non_rule_file_at_the_root` |
| M-decode-12 | Missed | `decode.rs:666:5: replace skipped -> bool with false` | U4 `a_case_ignores_node_modules_and_dot_directories` |
| M-decode-13 | Missed | `decode.rs:669:35: replace \|\| with && in skipped` | U4 `a_case_ignores_node_modules_and_dot_directories` |
| M-decode-14 | Missed | `decode.rs:725:31: replace + with * in parse_markdown` | U5 `a_heading_inside_an_example_block_stays_in_the_message` |
| M-decode-15 | Missed | `decode.rs:725:31: replace + with - in parse_markdown` | U5 `a_heading_inside_an_example_block_stays_in_the_message` |
| M-decode-16 | Missed | `decode.rs:737:28: replace + with * in parse_markdown` | U5 `a_markdown_rule_with_two_grit_blocks_is_refused`; U5 `a_rule_whose_example_block_precedes_the_grit_block_keeps_its_body` |
| T-decode-17 | Timeout | `decode.rs:737:28: replace + with - in parse_markdown` | stays a Timeout, counted as detected (Q5); U5 `a_rule_whose_example_block_precedes_the_grit_block_keeps_its_body` fails fast under it, after `bundled_packs` has hung |
| T-decode-18 | Timeout | `decode.rs:743:16: replace += with *= in parse_markdown` | stays a Timeout, counted as detected (Q5) |
| M-domain-01 | Missed | `domain.rs:49:9: replace NonEmpty<T>::is_empty -> bool with true` | U9 anchored `exclude_re` (`clippy::len_without_is_empty`): moved by the U8 decision rule, since `len` has a reader, `SelectedFiles::total` (`domain.rs:696`, rustc E0599) (Q4) |
| M-domain-02 | Missed | `domain.rs:97:9: replace <impl fmt::Display for ParamName>::fmt -> fmt::Result with Ok(Default::default())` | Deleted in U8 (`a4589f4f1`): `Display for ParamName` (Q4) |
| M-domain-03 | Missed | `domain.rs:103:9: replace <impl fmt::Display for ParamNameRefusal>::fmt -> fmt::Result with Ok(Default::default())` | U8 carries the refusal reason into `InvalidParamName`; U1 `a_pack_parameter_that_is_not_an_identifier_refuses_naming_the_key_the_config_and_the_reason` (Q4) |
| M-domain-04 | Missed | `domain.rs:179:9: replace RuleId::scope -> &str with ""` | U5 `a_rule_cannot_call_a_pattern_declared_in_another_scope` |
| M-domain-05 | Missed | `domain.rs:179:9: replace RuleId::scope -> &str with "xyzzy"` | U5 `a_rule_cannot_call_a_pattern_declared_in_another_scope` |
| M-domain-06 | Missed | `domain.rs:212:9: replace <impl fmt::Display for RelPathRefusal>::fmt -> fmt::Result with Ok(Default::default())` | U7 `relpath_refusals.rs: an_absolute_path_refuses_with_the_reason` |
| M-domain-07 | Missed | `domain.rs:292:9: replace <impl fmt::Display for ReasonCode>::fmt -> fmt::Result with Ok(Default::default())` | U6 `a_violation_carries_the_rule_path_line_and_description (extend)` |
| M-domain-08 | Missed | `domain.rs:443:9: replace <impl fmt::Display for SourcePath>::fmt -> fmt::Result with Ok(Default::default())` | Deleted in U8 (`a4589f4f1`): `Display for SourcePath` (Q4) |
| M-domain-09 | Missed | `domain.rs:529:9: replace ParsedRule::title -> &str with ""` | Deleted in U8 (`a4589f4f1`): `ParsedRule::title` and the `title` field (Q4) |
| M-domain-10 | Missed | `domain.rs:529:9: replace ParsedRule::title -> &str with "xyzzy"` | Deleted in U8 (`a4589f4f1`): `ParsedRule::title` and the `title` field (Q4) |
| M-domain-11 | Missed | `domain.rs:559:9: replace ParsedRule::params -> &BTreeMap<ParamName, String> with Box::leak(Box::new(BTreeMap::new()))` | Deleted in U8 (`a4589f4f1`):  the `ParsedRule::params` accessor (Q4) |
| M-domain-12 | Missed | `domain.rs:568:49: replace && with \|\| in ParsedRule::library_against` | U5 `a_rule_cannot_call_a_pattern_declared_in_another_scope` |
| M-domain-13 | Missed | `domain.rs:755:9: replace IgnoreGlobs::is_empty -> bool with false` | Deleted in U8 (`a4589f4f1`): `IgnoreGlobs::is_empty` (Q4) |
| M-domain-14 | Missed | `domain.rs:755:9: replace IgnoreGlobs::is_empty -> bool with true` | Deleted in U8 (`a4589f4f1`): `IgnoreGlobs::is_empty` (Q4) |
| M-domain-15 | Missed | `domain.rs:781:9: replace ScanPlan::files_selected -> usize with 0` | U6 `the_plan_counts_its_selected_files_and_rules` |
| M-domain-16 | Missed | `domain.rs:781:9: replace ScanPlan::files_selected -> usize with 1` | U6 `the_plan_counts_its_selected_files_and_rules` |
| M-domain-17 | Missed | `domain.rs:786:9: replace ScanPlan::rule_count -> usize with 0` | U6 `the_plan_counts_its_selected_files_and_rules` |
| M-domain-18 | Missed | `domain.rs:786:9: replace ScanPlan::rule_count -> usize with 1` | U6 `the_plan_counts_its_selected_files_and_rules` |
| M-domain-19 | Missed | `domain.rs:804:9: replace Execution::language -> TargetLanguage with Default::default()` | U6 `an_execution_names_its_language_and_reads_only_its_own_language` |
| M-domain-20 | Missed | `domain.rs:808:9: replace Execution::files -> impl Iterator<Item =&InputFile> with ::std::iter::empty()` | U6 `an_execution_names_its_language_and_reads_only_its_own_language` |
| M-domain-21 | Missed | `domain.rs:812:9: replace Execution::read -> impl Iterator<Item =&InputFile> with ::std::iter::empty()` | U6 `an_execution_names_its_language_and_reads_only_its_own_language` |
| M-domain-22 | Missed | `domain.rs:958:9: replace CaseKind::directory -> &'static str with ""` | U4 `a_rule_with_only_good_cases_fails_as_untested (extend)` |
| M-domain-23 | Missed | `domain.rs:958:9: replace CaseKind::directory -> &'static str with "xyzzy"` | U4 `a_rule_with_only_good_cases_fails_as_untested (extend)` |
| M-domain-24 | Missed | `domain.rs:1104:9: replace EmbeddedFile::path -> &str with ""` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-domain-25 | Missed | `domain.rs:1104:9: replace EmbeddedFile::path -> &str with "xyzzy"` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-domain-26 | Missed | `domain.rs:1109:9: replace EmbeddedFile::contents -> &str with ""` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-domain-27 | Missed | `domain.rs:1109:9: replace EmbeddedFile::contents -> &str with "xyzzy"` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-encode-01 | Missed | `encode.rs:79:5: replace report_schema -> Schema with Default::default()` | U3 `the_report_schema_is_the_committed_envelope_schema` |
| M-encode-02 | Missed | `encode.rs:203:5: replace concise_line -> String with "xyzzy".into()` | U3 `a_concise_line_normalises_whitespace_and_renders_whole_under_its_byte_budget` |
| M-encode-03 | Missed | `encode.rs:203:5: replace concise_line -> String with String::new()` | U3 `a_concise_line_normalises_whitespace_and_renders_whole_under_its_byte_budget` |
| M-encode-04 | Missed | `encode.rs:211:14: replace + with * in concise_line` | U3 `a_concise_line_at_the_message_cutoff_drops_the_message` |
| M-encode-05 | Missed | `encode.rs:211:18: replace + with * in concise_line` | U3 `a_concise_line_at_the_message_cutoff_drops_the_message` |
| M-encode-06 | Missed | `encode.rs:211:35: replace < with <= in concise_line` | U3 `a_concise_line_at_the_message_cutoff_drops_the_message` |
| M-encode-07 | Missed | `encode.rs:211:35: replace < with == in concise_line` | U3 `a_concise_line_cuts_the_message_at_the_byte_budget_with_one_trailing_ellipsis` |
| M-encode-08 | Missed | `encode.rs:212:60: replace - with / in concise_line` | U3 `a_concise_line_cuts_the_message_at_the_byte_budget_with_one_trailing_ellipsis` |
| M-encode-09 | Missed | `encode.rs:215:86: replace + with * in concise_line` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| M-encode-10 | Missed | `encode.rs:215:86: replace + with - in concise_line` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| M-encode-11 | Missed | `encode.rs:220:5: replace one_line -> String with "xyzzy".into()` | U3 `a_concise_line_normalises_whitespace_and_renders_whole_under_its_byte_budget` |
| M-encode-12 | Missed | `encode.rs:224:5: replace cut_end -> String with String::new()` | U3 `a_concise_line_normalises_whitespace_and_renders_whole_under_its_byte_budget` |
| T-encode-13 | Timeout | `encode.rs:229:13: replace -= with /= in cut_end` | stays a Timeout, counted as detected (Q5); forced every run by U3 `a_concise_line_never_splits_a_multibyte_character` |
| M-encode-14 | Missed | `encode.rs:235:5: replace cut_start -> String with "xyzzy".into()` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| M-encode-15 | Missed | `encode.rs:235:5: replace cut_start -> String with String::new()` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| M-encode-17 | Missed | `encode.rs:238:32: replace - with / in cut_start` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| T-encode-16 | Timeout | `encode.rs:238:32: replace - with + in cut_start` | stays a Timeout, counted as detected (Q5); forced every run by U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| M-encode-18 | Missed | `encode.rs:239:11: delete ! in cut_start` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| M-encode-20 | Missed | `encode.rs:240:15: replace += with -= in cut_start` | U3 `a_concise_line_never_splits_a_multibyte_character` |
| T-encode-19 | Timeout | `encode.rs:240:15: replace += with *= in cut_start` | stays a Timeout, counted as detected (Q5); forced every run by U3 `a_concise_line_never_splits_a_multibyte_character` |
| M-encode-21 | Missed | `encode.rs:336:5: replace table_cell -> String with "xyzzy".into()` | U3 `a_github_summary_row_escapes_pipes_backslashes_and_newlines` |
| M-encode-22 | Missed | `encode.rs:341:5: replace render_fixtures -> String with "xyzzy".into()` | U4 `a_pack_with_a_bad_and_a_good_case_passes (extend)`; U4 `a_bad_case_that_yields_no_finding_names_the_rule_and_the_case (extend)` |
| M-encode-23 | Missed | `encode.rs:341:5: replace render_fixtures -> String with String::new()` | U4 `a_pack_with_a_bad_and_a_good_case_passes (extend)`; U4 `a_bad_case_that_yields_no_finding_names_the_rule_and_the_case (extend)` |
| M-engine-01 | Missed | `engine.rs:218:5: replace parse -> bool with false` | U7 `engine_contract.rs: a_file_with_syntax_errors_still_parses` |
| M-engine-02 | Missed | `engine.rs:218:5: replace parse -> bool with true` | U9 anchored `exclude_re` (Q3: equivalent) |
| M-error-01 | Missed | `error.rs:125:9: replace <impl std::fmt::Display for FrontmatterProblem>::fmt -> std::fmt::Result with Ok(Default::default())` | U5 `a_rule_without_valid_frontmatter_refuses_naming_the_file_and_the_key (extend)` |
| M-shell-01 | Missed | `shell.rs:79:5: replace is_file -> bool with false` | U1 `is_file_is_true_only_for_an_existing_regular_file` |
| M-shell-02 | Missed | `shell.rs:79:5: replace is_file -> bool with true` | U1 `is_file_is_true_only_for_an_existing_regular_file` |
| M-shell-03 | Missed | `shell.rs:92:56: replace != with == in collect_rules` | U5 `a_consumer_directory_loads_nested_rules_but_never_a_fixtures_subtree` |

### Timeouts

Logs are from `mutants.out-1-{1,36,40,44,50}/log/*.log`. cargo-mutants runs plain `cargo test` (`src/cargo.rs` `cargo_argv` adds no `--no-fail-fast`), so the test binaries run in name order (`bundled_packs`, `engine_contract`, `fixture_runner`, `laws`, ...), and the first binary that hangs turns the mutant into a Timeout.

**Ruling Q5 (confirmed):** all five stay Timeouts. A timeout means the suite did not pass on the mutant, which counts as detected. There is no `src` unit test to reorder test binaries, no production loop rewrite, and no exclusion.

- **T-decode-17** (`decode.rs:735:28` `+` -> `-`): **stays a Timeout.** The log hangs in `bundled_packs`. Every bundled rule markdown ends with a closing fence, and the mutant re-reads the line before each fence, so the walk never passes the last one. The U5 test `a_rule_whose_example_block_precedes_the_grit_block_keeps_its_body` ends on a prose line, which turns this mutant into a fast `RuleNoGritBlock` failure: the mutant re-reads the closing fence of the ```` ```json ```` block as an opener, whose close is the `grit` block's closing fence, so the `grit` opener is skipped (cursor 5 -> 6 -> 7 -> 13 -> 14 -> 16 -> 17 on that input). It runs in `rule_loading`, after `bundled_packs` has already hung, so the run still reports Timeout. Under Q5 it stays that way.
- **T-decode-18** (`decode.rs:743:16` `+=` -> `*=`): **stays a Timeout, and no assertion can catch it.** `lines[0]` is always the blanked frontmatter line (`decode.rs:237-311`, called at `:200`), so on every valid markdown rule `cursor *= 1` spins at 0. The only inputs on which it terminates have every visited line a fence, and on those it makes the same transitions as `+= 1`. So it either hangs or produces identical output.
- **T-encode-13** (`encode.rs:229:13` `-=` -> `/=`), **T-encode-16** (`encode.rs:238:32` `-` -> `+`), **T-encode-19** (`encode.rs:240:15` `+=` -> `*=`): **stay Timeouts, and no assertion can catch them.** The mutated statement runs only when the index is not a char boundary, and the mutation then either leaves the index unchanged (`/= 1`, `*= 1`) or moves it past the end, where no boundary exists. Otherwise the output is byte-identical. Today they time out in `laws` only when proptest generates a long path or a multi-byte cut. The U3 tests `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` and `a_concise_line_never_splits_a_multibyte_character` force those inputs every run. These are behaviour tests that kill other mutants in their own right, and as a side effect the three Timeouts become deterministic instead of depending on the proptest seed. `encode_contract` sorts before `laws`, so the hang moves earlier. The cost is unchanged: one per-mutant timeout each.

### Equivalent

- **M-engine-02** (`engine.rs:218:5` `parse -> bool` with `true`). `engine::parse` is `language.get_parser().parse_file(..).is_some()` (`engine.rs:217-228`). It is called with the execution's language, and only for files whose language key equals the rule's (`decide.rs:297-299` `Task::reads`; bench `breakdown.rs:152-154`). A file's language comes from its own extension (`decode.rs:703-706`). So every pair that reaches `parse` is a file parsed by its own extension's language, and a `.vue` file always arrives as Vue. At gritql `4ca2834` that path runs `MarzanoParser::parse_file` (`crates/language/src/language.rs:253-268`). It returns `None` only when tree-sitter's `parse` returns `None`/`Err`, or when `file_parsing_error` fails. `file_parsing_error` (`:510-544`) always returns `Ok`: ERROR/MISSING nodes become logs, not a failure. The CSS and JS-like parsers do have `None` branches, but only for a `.vue` path parsed with a non-Vue language, which this pipeline cannot produce. So `parse` is constant `true` on every reachable input. U7's `engine_contract` test kills only its sibling M-engine-01 (`false`).
- **Ruling Q3:** exclude it with one anchored `exclude_re` in `.cargo/mutants.toml` (U9). This is the baseline row `mutation-discipline/unkillable-survivors-are-dead-code.md` allows: an equivalent mutant on live code (`crates/gritlint_bench/src/breakdown.rs:154` reads it) that no rewrite removes, since returning `()` or an `Option` only creates a new unkillable mutant.

### Acceptance Examples

- AE1. **Covers R1.** Given the U6 tie test, with `a/package.json` and `a/tsconfig.json` both `{\n  "probe": 1,\n  "vite": "1",\n  "compilerOptions": {}\n}\n` and SIBLING_JOIN with `locate: `"probe": $_``, when scanned, then the location is `local/join a/package.json:2:3-2:13`. Under M-decide-02 it would be `a/tsconfig.json:2:3-2:13`, because the engine returns results sorted (gritql `crates/core/src/problem.rs:330` `results.sort()`; `MatchResult`/`Match` derive `Ord` in field order messages, variables, source_file, ...; `api.rs:14-36,346-368`). With identical contents, messages, variables and ranges are equal, so `source_file` decides, and the mutant's fold keeps the last result.
- AE2. **Covers R1.** Given U1's schema test, when `GritlintConfig::schema_for_packs(&decode::embedded_packs(<packs/ as EmbeddedFile, as apps/gritlint/src/main.rs:186-197 builds them>))` is pretty-printed with a trailing newline, then it equals `npm/gritlint/configuration_schema.json` byte for byte, which is what `gritlint schema` prints (`main.rs:104-114`).
- AE3. **Covers R5, R7, R8.** Given the post-merge per-push Mutants run on `main` (a full run, Q1), when it finishes, then every Missed row the table assigns a test is Caught, M-engine-02 is not listed (U9), the deleted accessors' mutants are not listed (U8), and the timeout list holds exactly the five Timeout rows.

### Scope Boundaries

- Test files under `crates/gritlint_core/tests/` (U1-U7). Production code changes only in U8: the Q4 deletions and the `InvalidParamName` reason, as one `refactor(core): ...` commit with a patch changeset. `.cargo/mutants.toml` gains only U9's anchored `exclude_re` line(s). `.compound-engineering/config.yaml` changes only in U0. No workflow change.
- No new dev-dependency. `serde_json`, `tempfile` and `proptest` are already available to the tests (`crates/gritlint_core/Cargo.toml:10-27`).
- No local mutation of any kind. The only cargo-mutants command allowed is `--list`. No hand-applied mutant, and no scratch-copy edit to watch a test go red. Kills are argued by reading.
- No `src` unit test to reorder test binaries, no production loop rewrite to avoid a Timeout, and no Timeout exclusion (Q5).
- `scan_contract.rs` owns scanning and `rule_loading.rs` owns rule loading. Config decoding (`enabled_packs`, the config schema, config discovery via `shell::is_file`) and rendering (`render_concise`, `render_github_summary`, `report_schema`) have no owner, which is why `config_decode.rs` and `encode_contract.rs` are new.

### Dependencies / Assumptions

- cargo-mutants 27.1.0 runs `cargo test --package gritlint_core` without `--no-fail-fast`. A failure in any test binary therefore means Caught, and a hang in an earlier binary means Timeout.
- gritql `4ca2834` sorts `execute_files` results (`problem.rs:318-332`). AE1 relies only on the last ordering field that differs (`source_file`). If a gritql bump changes that, the U6 tie test still passes on correct code but may stop discriminating; the next full run would show M-decide-02 as Missed again (Risks).
- `npm/gritlint/configuration_schema.json` and `report_schema.json` are the published contracts. The flake checks `gritlint-schema` (`nix/gritlint-checks.nix:72-82`) keep them equal to the CLI output, so the U1 and U3 schema tests pass on `5c9c2ab21`. U8 changes no schema.
- U0 re-pins the compound packs to systemfsoftware `main` at `02bf9653b1ca2b0a2951baf2743bb3f70d19fc77`; `9154691d` predates `f2bfe45df4`. U0 also declares `mutation-discipline`, so from then on that pack is a declared rule cited by this plan and enforced by ce-code-review.

### Rulings (sub-conductor plan gate, 2026-10-11)

- Q1. **The proof run is the post-merge per-push Mutants run on `main`, as a full run, with no manual dispatch.** This PR changes `.cargo/mutants.toml` (U9), and `scripts/tools/mutants-plan.sh:6,66-72` makes that file a full-run trigger against the last green `main` SHA (`5c9c2ab21`). Rejected: (b) a prefix trigger for `crates/gritlint_core/tests/`, which would cost about 234 job-minutes on every test-only push; (c) a pre-merge branch dispatch, because a tests-only PR cannot regress `main`, a wrong kill argument becomes a follow-up test, and heavy runs stay postsubmit. Fallback, only if the root overrules Q3 (so `.cargo/mutants.toml` does not change): root dispatches `gh workflow run mutants.yml -R systemfsoftware/gritlint --ref main -f mode=full` after the merge.
- Q2. **Pin today's output.** KTD24 (`docs/plans/2026-10-10-1309-feat-gritlint-standalone-plan.md:192`) cuts the message first, then the start of the path. At `fixed` >= 196 the message is cut to zero bytes and branch 3 runs, the same branch every long-path line takes. That is the stated contract, not a bug, and U3's test doc comment says so.
- Q3. **Exclude M-engine-02** with one anchored `exclude_re` (U9; Equivalent section).
- Q4. **Delete what nothing reads, all nine rows, with no exception.** A test on an accessor nothing reads pins dead code, and `mutation-discipline/unkillable-survivors-are-dead-code.md` says delete it when nothing reads it at runtime. U8 deletes:
  - `ParsedRule::title`, and the `title` field if it is then unread;
  - the `ParsedRule::params` accessor;
  - `Display for ParamName` and `Display for SourcePath`;
  - `IgnoreGlobs::is_empty`;
  - `NonEmpty::len` and `NonEmpty::is_empty` together, if `len` has no reader.

  Decision rule (R6): a deletion that fails to compile names its reader, and that row gets a test through the reader instead. If `NonEmpty::len` has a reader, both stay, and M-domain-01 gets its own anchored `exclude_re` line in U9 citing `clippy::len_without_is_empty`. For `ParamNameRefusal`, the refusal reason is carried into `GritlintError::InvalidParamName` (`error.rs:36-37`) so the CLI prints why. U1's refusal test asserts that the message names the key, the config and the reason. A patch changeset records the message change. All of this is in this PR as its own commit, not a separate PR.
- Q5. **Confirmed.** All five Timeouts stay Timeouts (Timeouts section).
- A1. The first commit of the PR is U0.
- A2. U3 writes 200 and the 3-byte ellipsis as literals from the published contract and never imports the encode constants.
- A3. U2 compares each pack's sources with the **unfiltered** listing of `packs/<pack>/rules/`. At `5c9c2ab21` that is 15 `.md` files and nothing else, so a non-rule file added there later fails the test.
- A4. The Mutant identity gate (Verification Contract).

## Planning Contract

### Key Technical Decisions

- KTD1. **Oracles are committed contracts and the real tree, never a recomputation.** U1 compares the config schema with `npm/gritlint/configuration_schema.json`, and U3 compares the report schema with `report_schema.json`. U2 compares each pack's decoded sources with the unfiltered directory listing of `packs/<pack>/rules/`, and never re-applies the `starts_with && is_rule_file` filter it is testing (A3). U3's 200-byte budget and 3-byte ellipsis are literals from `npm/gritlint/README.md:86` (A2) (`schema-laws/tests-own-no-schemas`, `boundary-testing/real-system-oracles`).
- KTD2. **Enter at the public pipeline phases.** Tests call `shell::walk/read_tree/read_rule_sources/read_sources`, `decode::*`, `decide::plan/scan/executions/run_fixtures` and `encode::render_*`, as `main.rs` and the bench do (`cell-architecture/sandwich-phase-order`). There are no mocks and no private-function tests (`boundary-testing/no-mocks-on-internal-glue`).
- KTD3. **Assert what a consumer reads.** For example, M-decode-14/15 assert `ParsedRule::message()` (the rule description shown in reports), not the title, which U8 deletes.
- KTD4. **The M-decide-02 test does not depend on engine order.** The subagent's draft (three bubbles, `aaa/mmm/zzz` files with cross-named values) relied on how gritql orders `variables`. The existing test at `scan_contract.rs:545` shows why that fails: under `results.sort()` the `tsconfig.json` match (probe on line 2) sorts before `package.json` (probe on line 3), so "last wins" and "first path wins" coincide and the mutant survived. Byte-identical files make every field before `source_file` equal (AE1), so the mutant's choice is forced to the larger path (`boundary-testing/pin-dependency-semantics`: the dependency assumption is stated in the test's doc comment).
- KTD5. **One test per behaviour, not per mutant.** U2's single embedded-pack test kills seven. U6's executions test kills four. U3's long-path test kills six and forces one Timeout.
- KTD6. **Dead code is deleted, and the compiler finds the consumer** (`mutation-discipline/unkillable-survivors-are-dead-code.md`, declared from U0). U8 deletes each unread accessor. A compile error names a reader, which turns that row into a test through the reader. An equivalent mutant on live code gets one anchored, commented `exclude_re` line and nothing broader (U9).

### Risks

| Risk | Consequence | Handling |
| --- | --- | --- |
| A new test hangs under a mutant that a later binary catches today | That mutant moves from Caught to Timeout | Only U3's two cut tests can hang (in the three loops above, which are already Timeouts), and U5's T-decode-17 input fails fast. The post-merge full run's timeout list is compared with the five rows (AE3). |
| A gritql bump changes result ordering | The M-decide-02 test passes but stops discriminating | The doc comment names the assumption. The next full run shows M-decide-02 Missed. |
| A Q4 deletion has a reader outside the workspace | A downstream consumer of `gritlint_core` breaks | `gritlint_core` is not published on its own (the launcher is the versioned surface), and the patch changeset records the message change. A reader inside the workspace stops the deletion at compile time (R6). |
| The `exclude_re` matches more than M-engine-02 | A killable mutant is silently skipped | The regex is anchored on file, line:col shape and the full description, and U9 checks that `--list` loses exactly one name. |
| A kill argument is wrong (no local mutation is allowed) | The proof run still shows that mutant Missed | Every argument is written next to its test in U1-U7. A Missed row in the proof run is fixed by a follow-up test. No retry by hand-mutation. |

## Implementation Units

Commit order: U0 first, then U8 (its own `refactor(core): ...` commit), then U1-U7 and U9. The units are file-disjoint. U1's refusal test depends on U8's message. After U8, regenerate the table's line:col from `cargo mutants --workspace --list` at the head (list only). Helpers: reuse the target file's own `write`/`rule`/`located_rule`/`plan`/`scan`/`locations`/`make_pack`/`sources`/`ids`/`run_fixtures`. Where U1 and U3 need the same helper as an existing file, they copy it rather than create a shared module, because each `tests/*.rs` is its own crate. All IDs refer to the mutant table.

### U0. Compound packs re-pin and enrolment (`.compound-engineering/config.yaml`)

- Set `ref` to `02bf9653b1ca2b0a2951baf2743bb3f70d19fc77` (current systemfsoftware `main`) and add `mutation-discipline` to `pack:`, giving `[boundary-testing, cell-architecture, mutation-discipline, package-topology, schema-laws]`. #7 left the pack out only because the repo had no mutation reports yet. This is the first commit of the PR (A1).
- Check: the ce packs resolver (`packs-resolve.py`) resolves five roots with no errors.

### U1. Config decoding and config discovery (`tests/config_decode.rs`, new)

- `the_configuration_schema_is_the_committed_schema`: read `packs/` with `shell::read_tree`, map each file to `EmbeddedFile::new(<relative path with '/'>, contents)` as `main.rs:186-197` does, then `serde_json::to_string_pretty(&GritlintConfig::schema_for_packs(&decode::embedded_packs(&files)?))` + `"\n"` == `include_str!("../../../npm/gritlint/configuration_schema.json")`. Kills M-decode-01 (`schema()` -> `Default::default()` leaves no `properties` to extend) and M-decode-02.
- `a_config_pack_entry_becomes_a_typed_pack_name_and_parameters`: `GritlintConfig::parse(r#"{"packs": {"npm-provenance": {"repositoryUrl": "https://example.com/repo"}}}"#)`, then `decode::enabled_packs(&config, Path::new("gritlint.json"))` is one entry `npm-provenance` -> {`repositoryUrl` -> the URL}. Kills M-decode-03.
- `a_pack_parameter_that_is_not_an_identifier_refuses_naming_the_key_the_config_and_the_reason` (after U8): the same config with key `not-a-name` returns `GritlintError::InvalidParamName`, whose `to_string()` names `not-a-name`, `gritlint.json` and the reason `the name is not a GritQL identifier`, written as a literal. Kills M-decode-03 from the refusal side (`schema-laws/refusals-beside-generated-laws`) and M-domain-03.
- `is_file_is_true_only_for_an_existing_regular_file`: in a tempdir, a written `gritlint.json` is true; the directory itself, an absent path, and the file after `fs::remove_file` are all false. This is the CLI's config-discovery decision (`main.rs:122`). Kills M-shell-01/02.

### U2. Bundled packs (`tests/bundled_packs.rs`, extend)

- `every_bundled_pack_decodes_into_its_committed_rule_files`: decode the same `EmbeddedFile` list as U1. Assert the pack names are `[cell-architecture, npm-provenance, source-resolution, typecheck-build-mode]` (the `packs/` directories). For each pack, `sources()` paths equal the **unfiltered**, sorted `fs::read_dir(packs/<pack>/rules)` listing, prefixed `<pack>/rules/`: no extension filter (A3; 15 `.md` files at `5c9c2ab21`). Each source's `text()` equals the file's bytes, and each `required_params()` equals its `pack.json` `requiredParams`. Kills M-decode-04 (no packs), M-decode-05 (the first non-manifest file is parsed as the manifest, so `PackManifestParse`), M-decode-06 (other packs' rules and READMEs leak in), M-domain-24/25 (no path has a `/`, so no pack) and M-domain-26/27 (manifest contents `""`/`"xyzzy"` fail to parse).

### U3. Rendering (`tests/encode_contract.rs`, new)

Shared values: `Finding::new(RuleId::local(RuleName::new("concise-test")), Location::new(RelPath::new(path)?, None), message, Arc::new(Guidance::new(ReasonCode::new("generated")?, "Fix it.".into())))` and `Outcome::new(vec![finding], counts, 3)`. The head is `local/concise-test generated` (28 bytes). The budget is 200 bytes a line and the ellipsis `…` is 3 bytes, written as literals from the published contract (`npm/gritlint/README.md:86`; KTD24 in `docs/plans/2026-10-10-1309-feat-gritlint-standalone-plan.md:192`). The test never imports `encode::CONCISE_LINE_BYTES` or `ELLIPSIS`: an expected value never comes from the code under test (A2). Every length below is derived from these literals, and each exact expected string is written out in the test.

- `a_concise_line_normalises_whitespace_and_renders_whole_under_its_byte_budget`: message `"Fix  the\tthing now"` renders exactly `"a/package.json local/concise-test generated Fix the thing now\n1 finding(s) in 3 file(s) selected\n"`. Kills M-encode-02/03/11/12.
- `a_concise_line_cuts_the_message_at_the_byte_budget_with_one_trailing_ellipsis`: path `a/package.json` (14 bytes) and message `"a"*300` give a 200-byte line: path, head, `"a"*153` and `…`. Kills M-encode-07/08.
- `a_concise_line_at_the_message_cutoff_drops_the_message` (Q2: pins today's output): path `"pkg/" + "d"*158 + ".json"` (167 bytes, so `fixed` = 196) gives exactly `path + " " + head` (196 bytes). Kills M-encode-04/05/06, whose output is `path head …` (200 bytes). Doc comment: KTD24 cuts the message first, then the start of the path. At `fixed` >= 196 the message is cut to zero bytes and the line takes branch 3, the same branch every long-path line takes, so this is the stated contract, not a bug.
- `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name`: path `"d/" + "d"*244 + "/package.json"` (259 bytes) gives exactly `"…" + path[91..] + " " + head` (200 bytes). Kills M-encode-09/10/14/15/17/18 and forces T-encode-16.
- `a_concise_line_never_splits_a_multibyte_character`: (a) path `a/package.json` and message `"a"*152 + "é" + "z"*50` give the 199-byte line ending `"a"*152 + "…"`; (b) path `"pkg/" + "d"*77 + "é" + "d"*167` gives `"…" + path[83..] + " " + head`. Kills M-encode-20 and forces T-encode-13 and T-encode-19.
- `a_github_summary_row_escapes_pipes_backslashes_and_newlines`: path `a/|weird|.json`, message `"Fix the\n|table| \\ row"`, counts `[local/concise-test -> 2]`: `render_github_summary` equals the exact markdown, with `\|` and `\\` escaped and the newline collapsed. Kills M-encode-21 (and M-encode-11 again).
- `the_report_schema_is_the_committed_envelope_schema`: `serde_json::to_string_pretty(&encode::report_schema())` + `"\n"` == `include_str!("../../../npm/gritlint/report_schema.json")`. Kills M-encode-01.

### U4. Fixture runner (`tests/fixture_runner.rs`, extend)

- `a_consumer_layout_ignores_a_non_rule_file_at_the_root`: a tempdir with `no-console.grit`, `notes.txt` and one good case; `run_fixtures` is Ok, with one result for `local/no-console`. Kills M-decode-11 (`notes.txt` is parsed as a rule, so `RuleNoLanguage`).
- `a_case_ignores_node_modules_and_dot_directories`: a pack case whose `bad/basic` and `good/basic` each also hold `node_modules/dep.json` and `.cache/scratch.json` with `{"console": "debug"}`; the run `passed()`. Kills M-decode-12/13 (the good case gains findings).
- Extend `a_rule_with_only_good_cases_fails_as_untested`: assert `failures[0].case()` is `fixtures/no-console/bad`, and that `encode::render_fixtures` contains `FAIL bad pack-one/no-console fixtures/no-console/bad: no known-bad case`. Kills M-domain-22/23.
- Extend `a_pack_with_a_bad_and_a_good_case_passes` and `a_bad_case_that_yields_no_finding_names_the_rule_and_the_case`: assert `render_fixtures` equals the exact `ok`/`FAIL` lines plus `2 case(s) passed` or `1 case(s) failed`. Kills M-encode-22/23.

### U5. Rule loading (`tests/rule_loading.rs`, extend)

- `a_consumer_directory_with_two_rules_loads_both`: `consumer/{a,b}.grit` load as `[local/a, local/b]`. Kills M-decode-07 (dedup drops the second rule).
- `a_pack_ignores_a_non_rule_file_in_its_rules_directory`: `pack-one/rules/{react.grit, LICENSE}` gives one source, `rules/react.grit`. Kills M-decode-08/10.
- `a_heading_inside_an_example_block_stays_in_the_message`: markdown `FRONTMATTER + "\n```\n# Example heading\n```\n\nProse that describes the rule.\n\n```grit\nlanguage json\n`\"console\": $_`\n```\n"`; `message()` == "```\n# Example heading\n```\n\nProse that describes the rule." (KTD3). `lines[4]` is not a fence, so M-decode-15's `cursor - 1` cannot loop. Kills M-decode-14/15.
- `a_markdown_rule_with_two_grit_blocks_is_refused`: two `grit` blocks give `RuleNoGritBlock`, naming the file. Kills M-decode-16.
- `a_rule_whose_example_block_precedes_the_grit_block_keeps_its_body`: a pack rule with a ```` ```json ```` example block, then prose, then the `grit` block, then a final prose line (not a fence), plus bad/good cases and `expected-locations.txt`. `run_fixtures` passes, with result id `pack-one/example`. Kills M-decode-16; T-decode-17 stays a Timeout (Timeouts section).
- `a_rule_cannot_call_a_pattern_declared_in_another_scope`: pack `shared` rule `helper.grit` declares and uses `pattern shared_helper()`; consumer `caller.grit` calls `shared_helper()`. `load_rules` refuses with `RuleCompile`, whose message names `local/caller`. Kills M-domain-04/05/12. The assertion names `local/caller` because under M-domain-12 each rule also self-includes its own body, so a bare `is_err()` could pass for the wrong rule.
- `a_consumer_directory_loads_nested_rules_but_never_a_fixtures_subtree`: `consumer/nested/deep.grit` and `consumer/fixtures/hidden.grit` load as `[local/deep]`. Kills M-shell-03.
- Extend `a_rule_without_valid_frontmatter_refuses_naming_the_file_and_the_key`: per row, also assert the problem text: `is missing`, `is repeated`, `has no value`, ``is unknown; the keys are `reasonCode`, `nextAction`, `locate` ``, and ``must be lowercase kebab-case, such as `missing-provenance` ``. Kills M-error-01.


### U6. Scanning (`tests/scan_contract.rs`, extend)

- `a_walk_returns_only_files_with_a_language_extension`: `package.json`, `notes.txt` and `README.md` give `shell::walk` == `[package.json]`. The walked count is what the bench reports (`breakdown.rs:65-67`). Kills M-decode-09.
- Extend `a_violation_carries_the_rule_path_line_and_description`: `encode::render_human(&outcome)` contains `  test-reason: Fix it.`. Kills M-domain-07.
- `the_plan_counts_its_selected_files_and_rules`: two json files and two rules give `files_selected() == 2` and `rule_count() == 2`, the figures the bench reports (`breakdown.rs:133,135`). Kills M-domain-15..18.
- `an_execution_names_its_language_and_reads_only_its_own_language`: `pkg/package.json` + `pkg/tsdown.config.ts` with TSDOWN_PRESENCE_JOIN; `decide::executions` has length 1, `language().to_string() == "json"` (gritql `TargetLanguage` Display), sorted `files()` paths are both files, `read()` is `[pkg/package.json]`, and the tsdown `InputFile.contents` is empty. Kills M-decide-03 and M-domain-19/20/21.
- `a_multifile_locator_sees_only_the_files_the_rule_matched`: `a/extra.json` `{"probe": 1}`, `a/package.json` `{"vite": "1"}`, `a/tsconfig.json` `{"compilerOptions": {}}`, SIBLING_JOIN with `locate: `"probe": $_``; locations are `["local/join a/package.json"]` with no range. Kills M-decide-01 (under the mutant, `extra.json` reaches the locator and takes the finding).
- `a_multifile_locator_tie_goes_to_the_first_path_not_the_engine_order`: AE1. Kills M-decide-02 (KTD4).

### U7. Refusals and engine (`tests/relpath_refusals.rs`, `tests/engine_contract.rs`, extend)

- `relpath_refusals.rs` `an_absolute_path_refuses_with_the_reason`: `decode::select_files(vec!["/etc/passwd.json".into()], Path::new("/tmp/gritlint-root"))` gives `GritlintError::RelPath`, whose `to_string()` contains `the path is absolute`, the reason the CLI prints (`error.rs:75`). Kills M-domain-06.
- `engine_contract.rs` `a_file_with_syntax_errors_still_parses`: `engine::parse` of `{"a": 1}` and of the existing malformed fixture are both `true`, matching the diagnostics contract in `malformed_json_yields_a_parse_diagnostic_that_names_the_file`. Kills M-engine-01 only (M-engine-02 is equivalent and excluded by U9).

### U8. Delete unread code; the refusal reason reaches the CLI (`crates/gritlint_core/src/{domain,decode,error}.rs`, `.changeset/`)

One commit, `refactor(core): ...`, landing before U1's refusal test.

- Delete `ParsedRule::title`. If the `title` field is then unread, delete it and its constructor argument. `parse_markdown` keeps finding the title line, because the message excludes it (M-decode-14/15 assert `message()`). Kills M-domain-09/10 by deletion.
- Delete the `ParsedRule::params` accessor. `engine_parameters()` and `with_params()` read the field directly. Kills M-domain-11 by deletion.
- Delete `impl Display for ParamName` and `impl Display for SourcePath`. Kills M-domain-02/08 by deletion.
- Delete `IgnoreGlobs::is_empty`. Kills M-domain-13/14 by deletion.
- `NonEmpty::len` has a reader: deleting it fails with rustc E0599 at `domain.rs:696`, `SelectedFiles::total` (`self.by_language.values().map(NonEmpty::len).sum()`), which feeds `ScanPlan::files_selected`. Both `len` and `is_empty` stay, and U9 adds M-domain-01's own anchored `exclude_re` citing `clippy::len_without_is_empty`. Moved row: M-domain-01, from "U8 delete" to "U9 `exclude_re`".
- `GritlintError::InvalidParamName` gains the refusal (`reason: ParamNameRefusal`), and its message names the key, the config and the reason. The three `map_err(|_| ..)` sites (`decode.rs:114`, `:489`, `:591`) pass the refusal through. `ParamNameRefusal`'s Display now has a consumer, and U1's refusal test kills M-domain-03.
- `.changeset/<name>.md`: `"@systemfsoftware/gritlint": patch`, describing the clearer refusal message.
- Decision rule (R6): a deletion that fails to compile names its reader. That row then keeps its code and gets a test through the reader, and the table and this unit are updated in the same commit.
- After the commit, regenerate the table's line:col from `cargo mutants --workspace --list` at the head (list only).

### U9. Exclude the equivalent mutant (`.cargo/mutants.toml`)

- Add one anchored `exclude_re`, for example `'engine\.rs:[0-9]+:[0-9]+: replace parse -> bool with true$'`, with a comment above it: equivalent on every reachable input (gritql `4ca2834` `crates/language/src/language.rs:253-268` and `:510-544`; an execution only pairs a file with its own extension's language, `decide.rs` `Task::reads`); remove the line when a rule's language can differ from a scanned file's extension.
- U8 keeps `NonEmpty::len`, so a second anchored line excludes M-domain-01 (`replace NonEmpty<T>::is_empty -> bool with true$`), with its comment citing `clippy::len_without_is_empty`.
- Check with `cargo mutants --list` only: before and after, the listings differ by exactly the excluded name(s).

## Verification Contract

| Gate | Command or check | Proves |
| --- | --- | --- |
| Rust gate | `nix flake check` on the PR (CI `Nix · x86_64-linux`, `Nix · aarch64-linux`): fmt, clippy `-D warnings`, `cargo test`, the schema checks | All new tests pass on unmutated code; U8 compiles with no reader left |
| Packs | The ce packs resolver after U0: five roots, no errors | A1 |
| Mutant identity (A4) | `nix develop --command nix shell --inputs-from . nixpkgs#cargo-mutants --command cargo mutants --workspace --list` (list only, as `mutants.yml` invokes it) at the PR head | Every test-assigned row is present (matched by function and description, with the regenerated line:col); M-engine-02 is absent; the deleted accessors' mutants are absent |
| Exclusion | The same `--list` before and after U9 | Exactly the excluded name(s) disappear (R7) |
| Changeset | `.changeset/` holds U8's patch intent for `@systemfsoftware/gritlint` | The refusal-message change is versioned |
| PR job time | Jobs API wall time per PR job, recorded in the PR body | R4: every PR job at or under 10 minutes |
| Proof | The post-merge per-push Mutants run on `main` (full, triggered by `.cargo/mutants.toml`): its summary's caught/missed/timeout lists | AE3 |

## Definition of Done

- One PR from `test/kill-missed-mutants` into `main`, with CI green on its head and every job measured at or under 10 minutes. Its commits are U0 first, U8 as its own `refactor(core): ...` commit, then the tests and U9.
- The mutant table's line:col is regenerated after U8 from `cargo mutants --list` (list only).
- After the merge, the per-push full run on `main` reports every test-assigned Missed row Caught, lists neither M-engine-02 nor the deleted accessors' mutants, and holds exactly the five Timeout rows. Any row still Missed is reported with its test name, not re-argued.
