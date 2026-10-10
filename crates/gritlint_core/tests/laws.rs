#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use gritlint_core::decide;
use gritlint_core::decode;
use gritlint_core::domain::{
    Finding, Guidance, Language, Location, Outcome, ParsedRule, Position, Range, ReasonCode,
    RelPath, RuleCount, RuleId, RuleName, RuleSource, SelectedFile, SourcePath, Verdict,
};
use gritlint_core::encode;
use proptest::prelude::*;
use std::path::PathBuf;
use std::sync::Arc;

fn rel_path() -> impl Strategy<Value = String> {
    prop::collection::vec("[a-z0-9_-]{1,8}", 1..5).prop_map(|segments| segments.join("/"))
}

fn pattern() -> impl Strategy<Value = String> {
    prop_oneof![
        Just(".*".to_owned()),
        Just(r".*\.json".to_owned()),
        Just(r"[a-z0-9_-]+\.json".to_owned()),
    ]
}

fn rule_with(pattern: &str) -> ParsedRule {
    let body = format!(
        "---\nreasonCode: generated\nnextAction: Fix it.\n---\nlanguage json\nmultifile {{\n  file($name, $body) where {{ $name <: r\"{pattern}\" }}\n}}\n"
    );
    let source = RuleSource::new(SourcePath::new(PathBuf::from("generated.grit")), body);
    decode::parse_rule(None, &source).expect("the generated rule parses")
}

fn selected(path: &str) -> SelectedFile {
    let language = Language::from_extension("json").expect("the json grammar is compiled in");
    SelectedFile::new(language, RelPath::new(path).expect("a normalized path"))
}

fn guidance() -> impl Strategy<Value = Arc<Guidance>> {
    ("[a-z]{1,8}(-[a-z0-9]{1,8}){0,3}", "[a-zA-Zé:,%\n ]{0,80}").prop_map(|(code, action)| {
        Arc::new(Guidance::new(
            ReasonCode::new(&code).expect("a generated kebab-case code"),
            action,
        ))
    })
}

fn range() -> impl Strategy<Value = Option<Range>> {
    prop::option::of((1u32..500, 1u32..200, 0u32..3, 1u32..200).prop_map(
        |(line, column, extra, end_column)| Range {
            start: Position { line, column },
            end: Position {
                line: line + extra,
                column: end_column,
            },
        },
    ))
}

fn hostile_path() -> impl Strategy<Value = String> {
    prop::collection::vec("[a-z0-9_:,%é-]{1,40}", 1..10).prop_map(|segments| segments.join("/"))
}

fn finding() -> impl Strategy<Value = Finding> {
    (
        prop_oneof![Just("alpha"), Just("beta"), Just("local")],
        prop_oneof![rel_path(), hostile_path()],
        range(),
        "[a-zA-Zé:,%|\\\\\r\n ]{0,300}",
        guidance(),
    )
        .prop_map(|(rule, path, range, message, guidance)| {
            Finding::new(
                RuleId::local(RuleName::new(rule)),
                Location::new(RelPath::new(&path).expect("a normalized path"), range),
                message,
                guidance,
            )
        })
}

fn rule_count() -> impl Strategy<Value = RuleCount> {
    (
        prop_oneof![Just("alpha"), Just("beta"), Just("local")],
        0usize..10,
    )
        .prop_map(|(rule, files)| RuleCount::new(RuleId::local(RuleName::new(rule)), files))
}

proptest! {
    #[test]
    fn partition_assigns_each_scoped_file_to_one_directory(
        pattern in pattern(),
        paths in prop::collection::btree_set(rel_path(), 0..8),
    ) {
        let rule = rule_with(&pattern);
        let batch: Vec<SelectedFile> = paths.iter().map(|path| selected(path)).collect();
        let directories = decide::partition_multifile(&rule, &batch);
        let pattern = regex::Regex::new(&pattern).expect("the generated pattern compiles");

        let mut seen = std::collections::BTreeSet::new();
        for directory in &directories {
            for file in directory.files() {
                let parent = file.path().parent();
                prop_assert_eq!(parent.as_ref(), directory.directory());
                prop_assert!(seen.insert(file.path().as_str().to_owned()));
            }
        }
        for file in &batch {
            let expected = pattern.is_match(file.path().as_str());
            let present = seen.contains(file.path().as_str());
            prop_assert_eq!(present, expected);
        }
    }

    #[test]
    fn encoding_is_order_independent(
        findings in prop::collection::vec(finding(), 0..8),
        counts in prop::collection::vec(rule_count(), 0..4),
        rotation in any::<usize>(),
    ) {
        let mut reversed = findings.clone();
        reversed.reverse();
        let len = reversed.len();
        let by = len.checked_rem(rotation.max(1)).unwrap_or(0);
        let shuffled: Vec<Finding> = reversed.iter().cycle().skip(by).take(len).cloned().collect();

        let canonical = decide::canonicalize(findings, counts.clone(), 0);
        let permuted = decide::canonicalize(shuffled, counts, 0);
        let human_canonical = encode::render_human(&canonical);
        let human_permuted = encode::render_human(&permuted);
        let json_canonical = encode::render_json(&canonical).expect("json");
        let json_permuted = encode::render_json(&permuted).expect("json");
        prop_assert_eq!(human_canonical, human_permuted);
        prop_assert_eq!(json_canonical, json_permuted);
    }

    #[test]
    fn exit_code_follows_findings(
        findings in prop::collection::vec(finding(), 0..6),
        counts in prop::collection::vec(rule_count(), 0..4),
        files_selected in 0usize..5,
    ) {
        let outcome = Outcome::new(findings.clone(), counts, files_selected);
        let expected = u8::from(!findings.is_empty());
        prop_assert_eq!(decide::exit_code(outcome.verdict()), expected);
        prop_assert_eq!(decide::exit_code(Verdict::Refused), 2);
    }

    #[test]
    fn relpath_round_trips_produced_paths(path in rel_path()) {
        let parsed = RelPath::new(&path).expect("a constructed relative path decodes");
        prop_assert_eq!(parsed.as_str(), path.as_str());
        let again = RelPath::new(parsed.as_str()).expect("re-decodes");
        prop_assert_eq!(again.as_str(), parsed.as_str());
    }

    #[test]
    fn relpath_refuses_absolute(path in rel_path()) {
        let absolute = format!("/{path}");
        prop_assert!(RelPath::new(&absolute).is_err());
    }

    #[test]
    fn relpath_refuses_parent_segments(path in rel_path()) {
        let trailing = format!("{path}/..");
        let leading = format!("../{path}");
        prop_assert!(RelPath::new(&trailing).is_err());
        prop_assert!(RelPath::new(&leading).is_err());
    }

    #[test]
    fn concise_output_stays_within_its_budget(
        findings in prop::collection::vec(finding(), 0..40),
        counts in prop::collection::vec(rule_count(), 0..4),
        files_selected in 0usize..5,
    ) {
        let shown = findings.len().min(encode::CONCISE_FINDINGS);
        let rest = findings.len() - shown;
        let outcome = decide::canonicalize(findings, counts, files_selected);
        let rendered = encode::render_concise(&outcome);

        let lines: Vec<&str> = rendered.lines().collect();
        prop_assert!(lines.len() <= encode::CONCISE_FINDINGS + 2, "{}", lines.len());
        prop_assert!(rendered.len() <= 4_500, "{}", rendered.len());
        for line in &lines {
            prop_assert!(line.len() <= encode::CONCISE_LINE_BYTES, "{} bytes: {line}", line.len());
        }
        let expected_lines = shown + usize::from(rest > 0) + 1;
        prop_assert_eq!(lines.len(), expected_lines);
        if rest > 0 {
            let remainder = format!(" and {rest} more;");
            prop_assert!(lines[shown].contains(&remainder), "{}", lines[shown]);
        }
    }

    #[test]
    fn github_output_is_one_parseable_command_per_finding(
        findings in prop::collection::vec(finding(), 0..10),
    ) {
        let outcome = decide::canonicalize(findings, Vec::new(), 1);
        let rendered = encode::render_github(&outcome);

        let lines: Vec<&str> = rendered.split('\n').filter(|line| !line.is_empty()).collect();
        prop_assert_eq!(lines.len(), outcome.findings().len());
        for (line, finding) in lines.iter().zip(outcome.findings()) {
            prop_assert!(!line.contains('\r'));
            let command = line.strip_prefix("::error ").expect("an error command");
            let (properties, _message) = command.split_once("::").expect("a message separator");
            let keys: Vec<&str> = properties
                .split(',')
                .map(|pair| pair.split_once('=').expect("key=value").0)
                .collect();
            let expected: &[&str] = if finding.location().range().is_some() {
                &["file", "line", "col", "endLine", "endColumn", "title"]
            } else {
                &["file", "title"]
            };
            prop_assert_eq!(keys, expected);
        }
    }

    #[test]
    fn github_summary_is_one_table_row_per_shown_finding_and_rule(
        findings in prop::collection::vec(finding(), 0..130),
        counts in prop::collection::vec(rule_count(), 0..4),
    ) {
        let total = findings.len();
        let shown = total.min(encode::SUMMARY_FINDINGS);
        let outcome = decide::canonicalize(findings, counts, 1);
        let rendered = encode::render_github_summary(&outcome);

        let rows: Vec<&str> = rendered
            .lines()
            .filter(|line| {
                line.starts_with('|')
                    && !line.split('|').all(|cell| matches!(cell.trim(), "" | "---"))
            })
            .collect();
        let headers = usize::from(shown > 0) + usize::from(!outcome.counts().is_empty());
        prop_assert_eq!(rows.len(), headers + shown + outcome.counts().len());
        let (finding_rows, count_rows) = rows.split_at(usize::from(shown > 0) + shown);
        for row in finding_rows {
            prop_assert_eq!(unescaped_pipes(row), 6, "{}", row);
        }
        for row in count_rows {
            prop_assert_eq!(unescaped_pipes(row), 3, "{}", row);
        }
        let remainder = format!(" and {} more;", total - shown);
        prop_assert_eq!(rendered.contains(&remainder), total > shown);
    }

    #[test]
    fn escaped_values_carry_no_command_delimiters_and_round_trip(text in "[a-zé%:,\r\n ]{0,40}") {
        let data = encode::escape_data(&text);
        let property = encode::escape_property(&text);

        prop_assert!(!data.contains(['\r', '\n']), "{data:?}");
        prop_assert!(!property.contains(['\r', '\n', ':', ',']), "{property:?}");
        prop_assert_eq!(unescape(&data)?, text.clone());
        prop_assert_eq!(unescape(&property)?, text);
    }
}

fn unescape(escaped: &str) -> Result<String, TestCaseError> {
    let mut pieces = escaped.split('%');
    let head = pieces.next().unwrap_or_default().to_owned();
    pieces.try_fold(head, |mut out, piece| {
        let (code, rest) = piece
            .get(..2)
            .zip(piece.get(2..))
            .ok_or_else(|| TestCaseError::fail(format!("a bare % in {escaped:?}")))?;
        out.push(match code {
            "25" => '%',
            "0D" => '\r',
            "0A" => '\n',
            "3A" => ':',
            "2C" => ',',
            _ => {
                return Err(TestCaseError::fail(format!(
                    "%{code} is no escape in {escaped:?}"
                )));
            }
        });
        out.push_str(rest);
        Ok(out)
    })
}

fn unescaped_pipes(row: &str) -> usize {
    row.chars()
        .fold((0, false), |(pipes, escaped), character| match character {
            _ if escaped => (pipes, false),
            '\\' => (pipes, true),
            '|' => (pipes + 1, false),
            _ => (pipes, false),
        })
        .0
}
