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

## Goal Capsule

- **Objective:** each of the 72 mutants that full run 38092194120 (`mode=full`, at `1b0437c7a`) reported as Missed is either caught by a `gritlint_core` test that drives real input through a public entry point and asserts an outcome a consumer can see, or is put to a ruling with its argument (Q3, Q4). Each of its 5 Timeouts has a stated disposition. Mutants flagged for a ruling instead of a test: one is equivalent (Q3), four have no consumer and no non-restating assertion (Q4), and the five Timeout dispositions, of which the contested one (T-decode-17) cannot be caught by an assertion in the current layout (Q5).
- **Means:** new and extended tests in `crates/gritlint_core/tests/` only (U1-U7). No production code changes, no change to `.cargo/mutants.toml` and no new dependency. Line numbers hold at `5c9c2ab21`: `git diff --stat 1b0437c7a 5c9c2ab21 -- crates apps Cargo.toml Cargo.lock .cargo` is empty.
- **Proof:** a Mutants run on `main` after the merge (Q1 decides which one). It must report Caught for every Missed row of the table below that names a test, and its timeout list must hold exactly the five Timeout rows.

## Product Contract

### Summary

Run 38092194120 tested all 475 mutants: 262 caught, 72 missed, 5 timeout and 136 unviable. Its survivor list was read from the run's `mutants.out-*` artifacts (`missed.txt`, `timeout.txt`, and the per-mutant `diff/` and `log/` files). Every survivor was then analysed against the code at `5c9c2ab21`, and its kill was argued from the mutant's diff alone, with no local mutation. 67 Missed mutants get a test. One Missed mutant (M-engine-02) is equivalent, and four (M-domain-01/03/13/14) have no consumer, so any test would restate their body (Q4). Of the 5 Timeouts, four are hang-or-identical by construction and one stays a Timeout because of how the tests are laid out (dispositions below).

### Problem Frame

These mutants survived for four reasons:

1. **The only consumer is outside the mutated package.** cargo-mutants runs only `gritlint_core`'s own tests (`--package`, cargo-mutants `book/src/workspaces.md:39-41`). The contracts below are checked today only by the `apps/gritlint` CLI, by the `crates/gritlint_bench` crate, or by Nix flake checks (`nix/gritlint-checks.nix:72-82`, schema drift). Affected: GritlintConfig::schema/schema_for_packs, report_schema, embedded_packs, executions, the ScanPlan and Execution accessors, `shell::is_file` and `engine::parse`.
2. **The concise and summary renderers are covered only by property laws** (`tests/laws.rs`). The laws bound the line length but do not pin which bytes are kept, so 20 encode mutants stay inside them.
3. **No test exercises the boundary.** Examples: a heading inside an example fence, two `grit` blocks, a non-rule file in `rules/`, `node_modules` inside a fixture case, a rule calling a pattern from another scope, and a frontmatter refusal whose wording is not asserted.
4. **There is no consumer at all.** Nine accessors and renderings have no caller in the workspace (Q4).

### Requirements

- R1. Every row whose Run outcome is Missed and whose last column names a test is caught by that test. A Timeout row's disposition is stated in the Timeouts section; the tests it names only make that Timeout deterministic. The argument is the mutant's diff applied to the stated input, without running the mutant. Each test asserts an outcome a consumer sees: rendered output, a returned plan/execution/pack value, a refusal message, or a CLI-visible decision. It never asserts calls, never restates the implementation, and never only re-reads a fixture. One exception, ruled under Q4: for the five no-consumer accessor rows (M-domain-02/08/09/10/11), the test asserts the accessor itself. For 09/10/11 that is a parse result (the heading and the bound parameter). For 02/08 it is the Display contract that a name or path renders as itself.
- R2. Tests use real input: tempdir trees, the committed `packs/` tree, and the committed `npm/gritlint/*.json` contracts. There are no mocks and no new fixture directories.
- R3. Tests follow the current layout: one file per contract under `crates/gritlint_core/tests/`. Two files are new, `config_decode.rs` and `encode_contract.rs`, because no existing contract file owns config decoding or rendering (the other files are the ones U2, U4, U5, U6 and U7 extend). The lint header is the existing `#![allow(...)]` block.
- R4. Every PR job stays at or under 10 minutes, measured from the jobs API on the PR head (`Nix · x86_64-linux`, `Nix · aarch64-linux`). Every added test is an in-process run over a tiny tree or a string. The one larger input is the `packs/` tree, which `bundled_packs.rs` already reads.
- R5. A Timeout is never hidden. Each of the 5 has a disposition (Timeouts section), and none is excluded through `.cargo/mutants.toml`.

### Mutant table

`ID` is the analysis id. Mutant names are cargo-mutants' names, relative to `crates/gritlint_core/src/`. Test names are the ones the units give.

| ID | Run outcome | Mutant (file:line:col: description) | Test |
| --- | --- | --- | --- |
| M-decide-01 | Missed | `decide.rs:174:50: replace && with \|\| in place` | U6 `a_multifile_locator_sees_only_the_files_the_rule_matched` |
| M-decide-02 | Missed | `decide.rs:186:39: replace match guard best.0 <= candidate.0 with false in place` | U6 `a_multifile_locator_tie_goes_to_the_first_path_not_the_engine_order` |
| M-decide-03 | Missed | `decide.rs:206:5: replace executions -> Result<Vec<Execution>, GritlintError> with Ok(vec![])` | U6 `an_execution_names_its_language_and_reads_only_its_own_language` |
| M-decode-01 | Missed | `decode.rs:53:9: replace GritlintConfig::schema -> Schema with Default::default()` | U1 `the_configuration_schema_is_the_committed_schema` |
| M-decode-02 | Missed | `decode.rs:58:9: replace GritlintConfig::schema_for_packs -> Schema with Default::default()` | U1 `the_configuration_schema_is_the_committed_schema` |
| M-decode-03 | Missed | `decode.rs:105:5: replace enabled_packs -> Result<BTreeMap<PackName, BTreeMap<ParamName, String>>, GritlintError> with Ok(BTreeMap::new())` | U1 `a_config_pack_entry_becomes_a_typed_pack_name_and_parameters`; U1 `a_pack_parameter_that_is_not_an_identifier_refuses_naming_the_key_and_the_config` |
| M-decode-04 | Missed | `decode.rs:144:5: replace embedded_packs -> Result<Vec<Pack>, GritlintError> with Ok(vec![])` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-decode-05 | Missed | `decode.rs:160:42: replace == with != in embedded_packs` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-decode-06 | Missed | `decode.rs:170:54: replace && with \|\| in embedded_packs` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-decode-07 | Missed | `decode.rs:354:49: replace == with != in load_rules` | U5 `a_consumer_directory_with_two_rules_loads_both` |
| M-decode-08 | Missed | `decode.rs:412:44: replace && with \|\| in rule_sources_in` | U5 `a_pack_ignores_a_non_rule_file_in_its_rules_directory` |
| M-decode-09 | Missed | `decode.rs:442:5: replace is_scannable -> bool with true` | U6 `a_walk_returns_only_files_with_a_language_extension` |
| M-decode-10 | Missed | `decode.rs:447:5: replace is_rule_file -> bool with true` | U5 `a_pack_ignores_a_non_rule_file_in_its_rules_directory` |
| M-decode-11 | Missed | `decode.rs:500:44: replace && with \|\| in consumer_layout` | U4 `a_consumer_layout_ignores_a_non_rule_file_at_the_root` |
| M-decode-12 | Missed | `decode.rs:664:5: replace skipped -> bool with false` | U4 `a_case_ignores_node_modules_and_dot_directories` |
| M-decode-13 | Missed | `decode.rs:667:35: replace \|\| with && in skipped` | U4 `a_case_ignores_node_modules_and_dot_directories` |
| M-decode-14 | Missed | `decode.rs:723:31: replace + with * in parse_markdown` | U5 `a_heading_inside_an_example_block_stays_in_the_message` |
| M-decode-15 | Missed | `decode.rs:723:31: replace + with - in parse_markdown` | U5 `a_heading_inside_an_example_block_stays_in_the_message` |
| M-decode-16 | Missed | `decode.rs:735:28: replace + with * in parse_markdown` | U5 `a_markdown_rule_with_two_grit_blocks_is_refused`; U5 `a_rule_whose_example_block_precedes_the_grit_block_keeps_its_body` |
| T-decode-17 | Timeout | `decode.rs:735:28: replace + with - in parse_markdown` | none: stays a Timeout (Q5); U5 `a_rule_whose_example_block_precedes_the_grit_block_keeps_its_body` fails fast under it, after `bundled_packs` has hung |
| T-decode-18 | Timeout | `decode.rs:743:16: replace += with *= in parse_markdown` | none (Timeout stays, Q5) |
| M-domain-01 | Missed | `domain.rs:49:9: replace NonEmpty<T>::is_empty -> bool with true` | none: no contract reads this value, and a test would restate the body (Q4) |
| M-domain-02 | Missed | `domain.rs:97:9: replace <impl fmt::Display for ParamName>::fmt -> fmt::Result with Ok(Default::default())` | U7 `laws.rs properties (Q4)` |
| M-domain-03 | Missed | `domain.rs:109:9: replace <impl fmt::Display for ParamNameRefusal>::fmt -> fmt::Result with Ok(Default::default())` | none: no contract reads this value, and a test would restate the body (Q4) |
| M-domain-04 | Missed | `domain.rs:185:9: replace RuleId::scope -> &str with ""` | U5 `a_rule_cannot_call_a_pattern_declared_in_another_scope` |
| M-domain-05 | Missed | `domain.rs:185:9: replace RuleId::scope -> &str with "xyzzy"` | U5 `a_rule_cannot_call_a_pattern_declared_in_another_scope` |
| M-domain-06 | Missed | `domain.rs:218:9: replace <impl fmt::Display for RelPathRefusal>::fmt -> fmt::Result with Ok(Default::default())` | U7 `relpath_refusals.rs: an_absolute_path_refuses_with_the_reason` |
| M-domain-07 | Missed | `domain.rs:298:9: replace <impl fmt::Display for ReasonCode>::fmt -> fmt::Result with Ok(Default::default())` | U6 `a_violation_carries_the_rule_path_line_and_description (extend)` |
| M-domain-08 | Missed | `domain.rs:443:9: replace <impl fmt::Display for SourcePath>::fmt -> fmt::Result with Ok(Default::default())` | U7 `laws.rs properties (Q4)` |
| M-domain-09 | Missed | `domain.rs:529:9: replace ParsedRule::title -> &str with ""` | U5 `an_enabled_pack_loads_its_rules_under_its_namespace (extend; Q4)` |
| M-domain-10 | Missed | `domain.rs:529:9: replace ParsedRule::title -> &str with "xyzzy"` | U5 `an_enabled_pack_loads_its_rules_under_its_namespace (extend; Q4)` |
| M-domain-11 | Missed | `domain.rs:559:9: replace ParsedRule::params -> &BTreeMap<ParamName, String> with Box::leak(Box::new(BTreeMap::new()))` | U5 `an_enabled_pack_loads_its_rules_under_its_namespace (extend; Q4)` |
| M-domain-12 | Missed | `domain.rs:593:49: replace && with \|\| in ParsedRule::library_against` | U5 `a_rule_cannot_call_a_pattern_declared_in_another_scope` |
| M-domain-13 | Missed | `domain.rs:755:9: replace IgnoreGlobs::is_empty -> bool with false` | none: no contract reads this value, and a test would restate the body (Q4) |
| M-domain-14 | Missed | `domain.rs:755:9: replace IgnoreGlobs::is_empty -> bool with true` | none: no contract reads this value, and a test would restate the body (Q4) |
| M-domain-15 | Missed | `domain.rs:811:9: replace ScanPlan::files_selected -> usize with 0` | U6 `the_plan_counts_its_selected_files_and_rules` |
| M-domain-16 | Missed | `domain.rs:811:9: replace ScanPlan::files_selected -> usize with 1` | U6 `the_plan_counts_its_selected_files_and_rules` |
| M-domain-17 | Missed | `domain.rs:816:9: replace ScanPlan::rule_count -> usize with 0` | U6 `the_plan_counts_its_selected_files_and_rules` |
| M-domain-18 | Missed | `domain.rs:816:9: replace ScanPlan::rule_count -> usize with 1` | U6 `the_plan_counts_its_selected_files_and_rules` |
| M-domain-19 | Missed | `domain.rs:834:9: replace Execution::language -> TargetLanguage with Default::default()` | U6 `an_execution_names_its_language_and_reads_only_its_own_language` |
| M-domain-20 | Missed | `domain.rs:838:9: replace Execution::files -> impl Iterator<Item =&InputFile> with ::std::iter::empty()` | U6 `an_execution_names_its_language_and_reads_only_its_own_language` |
| M-domain-21 | Missed | `domain.rs:842:9: replace Execution::read -> impl Iterator<Item =&InputFile> with ::std::iter::empty()` | U6 `an_execution_names_its_language_and_reads_only_its_own_language` |
| M-domain-22 | Missed | `domain.rs:988:9: replace CaseKind::directory -> &'static str with ""` | U4 `a_rule_with_only_good_cases_fails_as_untested (extend)` |
| M-domain-23 | Missed | `domain.rs:988:9: replace CaseKind::directory -> &'static str with "xyzzy"` | U4 `a_rule_with_only_good_cases_fails_as_untested (extend)` |
| M-domain-24 | Missed | `domain.rs:1134:9: replace EmbeddedFile::path -> &str with ""` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-domain-25 | Missed | `domain.rs:1134:9: replace EmbeddedFile::path -> &str with "xyzzy"` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-domain-26 | Missed | `domain.rs:1139:9: replace EmbeddedFile::contents -> &str with ""` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
| M-domain-27 | Missed | `domain.rs:1139:9: replace EmbeddedFile::contents -> &str with "xyzzy"` | U2 `every_bundled_pack_decodes_into_its_committed_rule_files` |
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
| T-encode-13 | Timeout | `encode.rs:229:13: replace -= with /= in cut_end` | U3 `a_concise_line_never_splits_a_multibyte_character` |
| M-encode-14 | Missed | `encode.rs:235:5: replace cut_start -> String with "xyzzy".into()` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| M-encode-15 | Missed | `encode.rs:235:5: replace cut_start -> String with String::new()` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| M-encode-17 | Missed | `encode.rs:238:32: replace - with / in cut_start` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| T-encode-16 | Timeout | `encode.rs:238:32: replace - with + in cut_start` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| M-encode-18 | Missed | `encode.rs:239:11: delete ! in cut_start` | U3 `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` |
| M-encode-20 | Missed | `encode.rs:240:15: replace += with -= in cut_start` | U3 `a_concise_line_never_splits_a_multibyte_character` |
| T-encode-19 | Timeout | `encode.rs:240:15: replace += with *= in cut_start` | U3 `a_concise_line_never_splits_a_multibyte_character` |
| M-encode-21 | Missed | `encode.rs:336:5: replace table_cell -> String with "xyzzy".into()` | U3 `a_github_summary_row_escapes_pipes_backslashes_and_newlines` |
| M-encode-22 | Missed | `encode.rs:341:5: replace render_fixtures -> String with "xyzzy".into()` | U4 `a_pack_with_a_bad_and_a_good_case_passes (extend)`; U4 `a_bad_case_that_yields_no_finding_names_the_rule_and_the_case (extend)` |
| M-encode-23 | Missed | `encode.rs:341:5: replace render_fixtures -> String with String::new()` | U4 `a_pack_with_a_bad_and_a_good_case_passes (extend)`; U4 `a_bad_case_that_yields_no_finding_names_the_rule_and_the_case (extend)` |
| M-engine-01 | Missed | `engine.rs:218:5: replace parse -> bool with false` | U7 `engine_contract.rs: a_file_with_syntax_errors_still_parses` |
| M-engine-02 | Missed | `engine.rs:218:5: replace parse -> bool with true` | none (equivalent, Q3) |
| M-error-01 | Missed | `error.rs:121:9: replace <impl std::fmt::Display for FrontmatterProblem>::fmt -> std::fmt::Result with Ok(Default::default())` | U5 `a_rule_without_valid_frontmatter_refuses_naming_the_file_and_the_key (extend)` |
| M-shell-01 | Missed | `shell.rs:79:5: replace is_file -> bool with false` | U1 `is_file_is_true_only_for_an_existing_regular_file` |
| M-shell-02 | Missed | `shell.rs:79:5: replace is_file -> bool with true` | U1 `is_file_is_true_only_for_an_existing_regular_file` |
| M-shell-03 | Missed | `shell.rs:92:56: replace != with == in collect_rules` | U5 `a_consumer_directory_loads_nested_rules_but_never_a_fixtures_subtree` |

### Timeouts

Logs are from `mutants.out-1-{1,36,40,44,50}/log/*.log`. cargo-mutants runs plain `cargo test` (`src/cargo.rs` `cargo_argv` adds no `--no-fail-fast`), so the test binaries run in name order (`bundled_packs`, `engine_contract`, `fixture_runner`, `laws`, ...), and the first binary that hangs turns the mutant into a Timeout.

- **T-decode-17** (`decode.rs:735:28` `+` -> `-`): **stays a Timeout.** The log hangs in `bundled_packs`. Every bundled rule markdown ends with a closing fence, and the mutant re-reads the line before each fence, so the walk never passes the last one. The U5 test `a_rule_whose_example_block_precedes_the_grit_block_keeps_its_body` ends on a prose line, which turns this mutant into a fast `RuleNoGritBlock` failure: the mutant re-reads the closing fence of the ```` ```json ```` block as an opener, whose close is the `grit` block's closing fence, so the `grit` opener is skipped (cursor 5 -> 6 -> 7 -> 13 -> 14 -> 16 -> 17 on that input). It runs in `rule_loading`, after `bundled_packs` has already hung, so the run still reports Timeout. Making it Caught needs a test binary that runs before `bundled_packs` (a unit test inside `src/decode.rs`, which breaks the layout) or a bounded loop in production code. Both are rulings (Q5).
- **T-decode-18** (`decode.rs:743:16` `+=` -> `*=`): **stays a Timeout, and no assertion can catch it.** `lines[0]` is always the blanked frontmatter line (`decode.rs:237-311`, called at `:200`), so on every valid markdown rule `cursor *= 1` spins at 0. The only inputs on which it terminates have every visited line a fence, and on those it makes the same transitions as `+= 1`. So it either hangs or produces identical output.
- **T-encode-13** (`encode.rs:229:13` `-=` -> `/=`), **T-encode-16** (`encode.rs:238:32` `-` -> `+`), **T-encode-19** (`encode.rs:240:15` `+=` -> `*=`): **stay Timeouts, and no assertion can catch them.** The mutated statement runs only when the index is not a char boundary, and the mutation then either leaves the index unchanged (`/= 1`, `*= 1`) or moves it past the end, where no boundary exists. Otherwise the output is byte-identical. Today they time out in `laws` only when proptest generates a long path or a multi-byte cut. The U3 tests `a_concise_line_cuts_the_start_of_a_long_path_keeping_the_file_name` and `a_concise_line_never_splits_a_multibyte_character` force those inputs every run. These are behaviour tests that kill other mutants in their own right, and as a side effect the three Timeouts become deterministic instead of depending on the proptest seed. `encode_contract` sorts before `laws`, so the hang moves earlier. The cost is unchanged: one per-mutant timeout each.

### Equivalent

- **M-engine-02** (`engine.rs:218:5` `parse -> bool` with `true`). `engine::parse` is `language.get_parser().parse_file(..).is_some()` (`engine.rs:217-228`). It is called with the execution's language, and only for files whose language key equals the rule's (`decide.rs:297-299`, bench `breakdown.rs:152-154`). A file's language comes from its own extension (`decode.rs:703-706`). So every pair that reaches `parse` is a file parsed by its own extension's language, and a `.vue` file always arrives as Vue. At gritql `4ca2834` that path runs `MarzanoParser::parse_file` (`crates/language/src/language.rs:253-268`). It returns `None` only when tree-sitter's `parse` returns `None`/`Err`, or when `file_parsing_error` fails. `file_parsing_error` (`:510-544`) always returns `Ok`: ERROR/MISSING nodes become logs, not a failure. A native parser with a language set returns a tree for any input. The CSS and JS-like parsers do have `None` branches, but only for a `.vue` path parsed with a non-Vue language, which this pipeline cannot produce. So `parse` is constant `true` on every reachable input, and no test can tell the mutant apart. It becomes killable once a rule's language can differ from a scanned file's extension. U7's `engine_contract` test kills only its sibling M-engine-01 (`false`). The disposition is a ruling (Q3).

### Acceptance Examples

- AE1. **Covers R1.** Given the U6 tie test, with `a/package.json` and `a/tsconfig.json` both `{\n  "probe": 1,\n  "vite": "1",\n  "compilerOptions": {}\n}\n` and SIBLING_JOIN with `locate: `"probe": $_``, when scanned, then the location is `local/join a/package.json:2:3-2:13`. Under M-decide-02 it would be `a/tsconfig.json:2:3-2:13`, because the engine returns results sorted (gritql `crates/core/src/problem.rs:330` `results.sort()`; `MatchResult`/`Match` derive `Ord` in field order messages, variables, source_file, ...; `api.rs:14-36,346-368`). With identical contents, messages, variables and ranges are equal, so `source_file` decides, and the mutant's fold keeps the last result.
- AE2. **Covers R1.** Given U1's schema test, when `GritlintConfig::schema_for_packs(&decode::embedded_packs(<packs/ as EmbeddedFile, as apps/gritlint/src/main.rs:186-197 builds them>))` is pretty-printed with a trailing newline, then it equals `npm/gritlint/configuration_schema.json` byte for byte, which is what `gritlint schema` prints (`main.rs:104-114`).
- AE3. **Covers R5.** Given the post-merge proof run (Q1), when it finishes, then its summary shows no test-assigned row as Missed, the Missed list holds only M-engine-02 and the Q4 rows, and the timeout list holds exactly the five Timeout rows.

### Scope Boundaries

- Test files only, under `crates/gritlint_core/tests/`. No production code, no `.cargo/mutants.toml` and no workflow change. Q2-Q5 may move a row into a production ruling. That is for the plan reviewer to decide, not ce-work.
- No new dev-dependency. `serde_json`, `tempfile` and `proptest` are already available to the tests (`crates/gritlint_core/Cargo.toml:10-27`).
- No local mutation of any kind: no `cargo mutants` run except `--list`, no hand-applied mutant, and no scratch edit to watch a test go red. Kills are argued by reading.
- `scan_contract.rs` owns scanning and `rule_loading.rs` owns rule loading. Config decoding (`enabled_packs`, the config schema, config discovery via `shell::is_file`) and rendering (`render_concise`, `render_github_summary`, `report_schema`) have no owner, which is why `config_decode.rs` and `encode_contract.rs` are new.

### Dependencies / Assumptions

- cargo-mutants 27.1.0 runs `cargo test --package gritlint_core` without `--no-fail-fast`. A failure in any test binary therefore means Caught, and a hang in an earlier binary means Timeout.
- gritql `4ca2834` sorts `execute_files` results (`problem.rs:318-332`). AE1 relies only on the last ordering field that differs (`source_file`). If a gritql bump changes that, the U6 tie test still passes on correct code but may stop discriminating; the next full run would show M-decide-02 as Missed again (Risks).
- `npm/gritlint/configuration_schema.json` and `report_schema.json` are the published contracts. The flake checks `gritlint-schema` (`nix/gritlint-checks.nix:72-82`) keep them equal to the CLI output today, so the U1 and U3 schema tests pass on `5c9c2ab21`.

### Outstanding Questions

- Q1. *(Resolve Before ce-work)* **The per-push run after the merge tests nothing.** This PR changes only `crates/gritlint_core/tests/*.rs`. A push runs `cargo mutants --in-diff` over `git diff <green> HEAD -- '*.rs'` (`.github/workflows/mutants.yml:101-107`). Tests carry no mutants, so the diff touches none, and cargo-mutants lists 0 and exits 0 (`in_diff.rs:47-57`). The plan job then starts no shard, and the run is green without having re-tested a single survivor. Options: (a) **recommended:** after the merge, root dispatches `gh workflow run mutants.yml -R systemfsoftware/gritlint --ref main -f mode=full` (the automation token gets 403 on dispatch). That is 53 shards, measured at 237 job-minutes and 52m55s wall on 38092194120, and it is the proof AE3 reads. (b) Make a change under `crates/gritlint_core/tests/` force a full run on push. Adding the directory to `FULL_RUN_TRIGGERS` is not enough: `mode()` in `scripts/tools/mutants-plan.sh` matches each trigger with `grep -qxF` against `git diff --name-only`, so only exact file paths match, and `mode()` needs a prefix match. Cost: about 237 job-minutes per such push, and it is a CI policy change. (c) Dispatch `mode=full` on `test/kill-missed-mutants` before the merge, then (a) after it. A wrong kill argument would then be fixed in the open PR instead of in a follow-up on `main`, at the cost of a second 237-job-minute run. (a) is recommended only because the brief names the run on `main` as the proof. (c) is the safer order if one more run is acceptable.
- Q2. *(Resolve Before ce-work)* **The concise line drops the message with no ellipsis when the prefix is 196 to 199 bytes.** `encode.rs:211` takes the message branch only when `fixed + 1 + 3 < 200`. For `fixed` in 196..=199 the location still fits, so the line is `location head` with the message silently gone. M-encode-04/05/06 move that boundary by one byte and print `location head …` instead, which arguably states the loss better. U3's `a_concise_line_at_the_message_cutoff_drops_the_message` pins today's output. Pin it (default), or rule it a bug and fix `encode.rs` first in a separate change, with the test written against the fix.
- Q3. *(Ruling)* M-engine-02 is equivalent (section above). Leave it Missed and record it, or exclude it in `.cargo/mutants.toml` (`exclude_re`). This plan does neither.
- Q4. *(Ruling)* **Nine mutants sit on public API with no caller in the workspace:** M-domain-01 `NonEmpty::is_empty`, M-domain-02 `ParamName` Display, M-domain-03 `ParamNameRefusal` Display (every caller discards it: `decode.rs:114`, `:489`, `:591` `map_err(|_| ..)`), M-domain-08 `SourcePath` Display, M-domain-09/10 `ParsedRule::title`, M-domain-11 `ParsedRule::params`, and M-domain-13/14 `IgnoreGlobs::is_empty`. The plan treats them in two groups:
  - *Tested by default (five):* M-domain-02/08 get a Display round trip (a name or path renders as itself, U7). M-domain-09/10/11 get U5's accessor assertions (the parsed heading and the bound parameter).
  - *No test (four):* M-domain-01 (the body is the literal `false`, `domain.rs:48-50`), M-domain-13/14 (the body is `self.0.is_empty()`, `domain.rs:753-756`) and M-domain-03 (a message nobody prints). Any assertion on these restates the mutated line, so the hard rule against restating the implementation rules it out.
  - Options for the four: (a) **recommended:** delete what nothing reads, or make it unconstructable, per `mutation-discipline/unkillable-survivors-are-dead-code.md`. That pack sits in the same pinned compound-packs source but is not declared in `.compound-engineering/config.yaml`, so it is cited here as evidence, not as a declared rule. Concretely: delete `IgnoreGlobs::is_empty`; drop `NonEmpty::len`/`is_empty` if nothing calls `len`, and otherwise keep `is_empty` and record it; carry the refusal reason into `InvalidParamName`'s message, which gives M-domain-03 a consumer and a test. These are public-surface changes needing a changeset, made in a separate production change, not in this test-only PR. `NonEmpty::is_empty` cannot be dropped alone while `len` exists (`clippy::len_without_is_empty` under `clippy::all = deny`, `Cargo.toml:47`). (b) Leave them Missed and record them, as M-engine-02 is under Q3. (c) Pin them anyway (rejected above).
- Q5. *(Ruling)* The Timeout dispositions above, in particular whether T-decode-17 should become Caught through a `src/decode.rs` unit test or a bounded loop. Recommended: leave all five as Timeouts.

## Planning Contract

### Key Technical Decisions

- KTD1. **Oracles are committed contracts and the real tree, never a recomputation.** U1 compares the config schema with `npm/gritlint/configuration_schema.json`, and U3 compares the report schema with `report_schema.json`. U2 compares each pack's decoded sources with a directory listing of `packs/<pack>/rules/`, and never re-applies the `starts_with && is_rule_file` filter it is testing (`schema-laws/tests-own-no-schemas`, `boundary-testing/real-system-oracles`).
- KTD2. **Enter at the public pipeline phases.** Tests call `shell::walk/read_tree/read_rule_sources/read_sources`, `decode::*`, `decide::plan/scan/executions/run_fixtures` and `encode::render_*`, as `main.rs` and the bench do (`cell-architecture/sandwich-phase-order`). There are no mocks and no private-function tests (`boundary-testing/no-mocks-on-internal-glue`).
- KTD3. **Assert what a consumer reads.** Where a mutant changes both a no-consumer accessor and a consumer-visible value, assert the visible value. For example, M-decode-14/15 assert `ParsedRule::message()` (the rule description shown in reports), not `title()`.
- KTD4. **The M-decide-02 test does not depend on engine order.** The subagent's draft (three bubbles, `aaa/mmm/zzz` files with cross-named values) relied on how gritql orders `variables`. The existing test at `scan_contract.rs:545` shows why that fails: under `results.sort()` the `tsconfig.json` match (probe on line 2) sorts before `package.json` (probe on line 3), so "last wins" and "first path wins" coincide and the mutant survived. Byte-identical files make every field before `source_file` equal (AE1), so the mutant's choice is forced to the larger path (`boundary-testing/pin-dependency-semantics`: the dependency assumption is stated in the test's doc comment).
- KTD5. **One test per behaviour, not per mutant.** U2's single embedded-pack test kills seven. U6's executions test kills four. U3's long-path test kills six and forces one Timeout.

### Risks

| Risk | Consequence | Handling |
| --- | --- | --- |
| A new test hangs under a mutant that a later binary catches today | That mutant moves from Caught to Timeout | Only U3's two cut tests can hang (in the three loops above, which are already Timeouts), and U5's T-decode-17 input fails fast. The post-merge full run's timeout list is compared with the five rows (AE3). |
| A gritql bump changes result ordering | The M-decide-02 test passes but stops discriminating | The doc comment names the assumption. The next full run shows M-decide-02 Missed. |
| Q2 rules the 196-byte drop a bug | U3's boundary test pins wrong output | Q2 is Resolve Before ce-work. |
| A kill argument is wrong (no local mutation is allowed) | The proof run still shows that mutant Missed | Every argument is written next to its test in U1-U7. A Missed row in the proof run is fixed by a follow-up test. No retry by hand-mutation. |

## Implementation Units

Each unit owns one to three test files. The units do not touch each other's files, so they can run in parallel in ce-work. Helpers: reuse the target file's own `write`/`rule`/`located_rule`/`plan`/`scan`/`locations`/`make_pack`/`sources`/`ids`/`run_fixtures`. Where U1 and U3 need the same helper as an existing file, they copy it rather than create a shared module, because each `tests/*.rs` is its own crate. All IDs refer to the mutant table.

### U1. Config decoding and config discovery (`tests/config_decode.rs`, new)

- `the_configuration_schema_is_the_committed_schema`: read `packs/` with `shell::read_tree`, map each file to `EmbeddedFile::new(<relative path with '/'>, contents)` as `main.rs:186-197` does, then `serde_json::to_string_pretty(&GritlintConfig::schema_for_packs(&decode::embedded_packs(&files)?))` + `"\n"` == `include_str!("../../../npm/gritlint/configuration_schema.json")`. Kills M-decode-01 (`schema()` -> `true` leaves no `properties` to extend) and M-decode-02.
- `a_config_pack_entry_becomes_a_typed_pack_name_and_parameters`: `GritlintConfig::parse(r#"{"packs": {"npm-provenance": {"repositoryUrl": "https://example.com/repo"}}}"#)`, then `decode::enabled_packs(&config, Path::new("gritlint.json"))` is one entry `npm-provenance` -> {`repositoryUrl` -> the URL}. Kills M-decode-03.
- `a_pack_parameter_that_is_not_an_identifier_refuses_naming_the_key_and_the_config`: the same config with key `not-a-name` returns `GritlintError::InvalidParamName`, whose `to_string()` names `not-a-name` and `gritlint.json`. Kills M-decode-03 from the refusal side (`schema-laws/refusals-beside-generated-laws`).
- `is_file_is_true_only_for_an_existing_regular_file`: in a tempdir, a written `gritlint.json` is true; the directory itself, an absent path, and the file after `fs::remove_file` are all false. This is the CLI's config-discovery decision (`main.rs:122`). Kills M-shell-01/02.

### U2. Bundled packs (`tests/bundled_packs.rs`, extend)

- `every_bundled_pack_decodes_into_its_committed_rule_files`: decode the same `EmbeddedFile` list as U1. Assert the pack names are `[cell-architecture, npm-provenance, source-resolution, typecheck-build-mode]` (the `packs/` directories). For each pack, `sources()` paths equal the sorted `fs::read_dir(packs/<pack>/rules)` entries with extension `md`/`grit`, prefixed `<pack>/rules/`. Each source's `text()` equals the file's bytes, and each `required_params()` equals its `pack.json` `requiredParams`. Kills M-decode-04 (no packs), M-decode-05 (the first non-manifest file is parsed as the manifest, so `PackManifestParse`), M-decode-06 (other packs' rules and READMEs leak in), M-domain-24/25 (no path has a `/`, so no pack) and M-domain-26/27 (manifest contents `""`/`"xyzzy"` fail to parse).

### U3. Rendering (`tests/encode_contract.rs`, new)

Shared values: `Finding::new(RuleId::local(RuleName::new("concise-test")), Location::new(RelPath::new(path)?, None), message, Arc::new(Guidance::new(ReasonCode::new("generated")?, "Fix it.".into())))` and `Outcome::new(vec![finding], counts, 3)`. The head is `local/concise-test generated` (28 bytes). `CONCISE_LINE_BYTES` is 200 and `…` is 3 bytes (`encode.rs:7-9`). Every length below is derived from these, and each exact expected string is written out in the test.

- `a_concise_line_normalises_whitespace_and_renders_whole_under_its_byte_budget`: message `"Fix  the\tthing now"` renders exactly `"a/package.json local/concise-test generated Fix the thing now\n1 finding(s) in 3 file(s) selected\n"`. Kills M-encode-02/03/11/12.
- `a_concise_line_cuts_the_message_at_the_byte_budget_with_one_trailing_ellipsis`: path `a/package.json` (14 bytes) and message `"a"*300` give a 200-byte line: path, head, `"a"*153` and `…`. Kills M-encode-07/08.
- `a_concise_line_at_the_message_cutoff_drops_the_message` (subject to Q2): path `"pkg/" + "d"*158 + ".json"` (167 bytes, so `fixed` = 196) gives exactly `path + " " + head` (196 bytes). Kills M-encode-04/05/06, whose output is `path head …` (200 bytes).
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
- *(Q4)* Extend `an_enabled_pack_loads_its_rules_under_its_namespace`: `title()` == `React dependency`, and `params()[condition]` is the bound value. Kills M-domain-09/10/11.

### U6. Scanning (`tests/scan_contract.rs`, extend)

- `a_walk_returns_only_files_with_a_language_extension`: `package.json`, `notes.txt` and `README.md` give `shell::walk` == `[package.json]`. The walked count is what the bench reports (`breakdown.rs:65-67`). Kills M-decode-09.
- Extend `a_violation_carries_the_rule_path_line_and_description`: `encode::render_human(&outcome)` contains `  test-reason: Fix it.`. Kills M-domain-07.
- `the_plan_counts_its_selected_files_and_rules`: two json files and two rules give `files_selected() == 2` and `rule_count() == 2`, the figures the bench reports (`breakdown.rs:133,135`). Kills M-domain-15..18.
- `an_execution_names_its_language_and_reads_only_its_own_language`: `pkg/package.json` + `pkg/tsdown.config.ts` with TSDOWN_PRESENCE_JOIN; `decide::executions` has length 1, `language().to_string() == "json"` (gritql `TargetLanguage` Display), sorted `files()` paths are both files, `read()` is `[pkg/package.json]`, and the tsdown `InputFile.contents` is empty. Kills M-decide-03 and M-domain-19/20/21.
- `a_multifile_locator_sees_only_the_files_the_rule_matched`: `a/extra.json` `{"probe": 1}`, `a/package.json` `{"vite": "1"}`, `a/tsconfig.json` `{"compilerOptions": {}}`, SIBLING_JOIN with `locate: `"probe": $_``; locations are `["local/join a/package.json"]` with no range. Kills M-decide-01 (under the mutant, `extra.json` reaches the locator and takes the finding).
- `a_multifile_locator_tie_goes_to_the_first_path_not_the_engine_order`: AE1. Kills M-decide-02 (KTD4).

### U7. Laws, refusals, engine (`tests/laws.rs`, `tests/relpath_refusals.rs`, `tests/engine_contract.rs`, extend)

- `relpath_refusals.rs` `an_absolute_path_refuses_with_the_reason`: `decode::select_files(vec!["/etc/passwd.json".into()], Path::new("/tmp/gritlint-root"))` gives `GritlintError::RelPath`, whose `to_string()` contains `the path is absolute`, the reason the CLI prints (`error.rs:75`). Kills M-domain-06.
- `engine_contract.rs` `a_file_with_syntax_errors_still_parses`: `engine::parse` of `{"a": 1}` and of the existing malformed fixture are both `true`, matching the diagnostics contract in `malformed_json_yields_a_parse_diagnostic_that_names_the_file`. Kills M-engine-01 only (M-engine-02 is equivalent).
- *(Q4)* `laws.rs` properties: for any identifier `s` matching `[A-Za-z_][A-Za-z0-9_]{0,15}`, `ParamName::new(s)?.to_string() == s` (M-domain-02). For any relative path `p`, `SourcePath::new(p).to_string() == p.display().to_string()` (M-domain-08).

## Verification Contract

| Gate | Command or check | Proves |
| --- | --- | --- |
| Rust gate | `nix flake check` on the PR (CI `Nix · x86_64-linux`, `Nix · aarch64-linux`): fmt, clippy `-D warnings`, `cargo test` | All new tests pass on unmutated code |
| Mutant identity | `nix develop --command nix shell --inputs-from . nixpkgs#cargo-mutants --command cargo mutants --workspace --list` (list only, as `mutants.yml` invokes it) at the PR head: the 77 names of the table are present, with unchanged line:col | The table still names real mutants |
| PR job time | Jobs API wall time per PR job, recorded in the PR body | R4: every PR job at or under 10 minutes |
| Proof | The post-merge Mutants run Q1 picks: its summary's caught/missed/timeout lists | AE3: every test-assigned Missed row Caught (Q4 may move rows to a production ruling), M-engine-02 per Q3, the five Timeouts unchanged |

## Definition of Done

- One PR from `test/kill-missed-mutants` into `main`, with CI green on its head and every job measured at or under 10 minutes.
- Q1-Q5 are ruled before ce-work starts, and the plan is updated with the rulings.
- After the merge, the proof run Q1 names reports every test-assigned Missed row Caught. Any row still Missed is reported with its test name, not re-argued.
