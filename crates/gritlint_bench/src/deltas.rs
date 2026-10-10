use std::collections::{BTreeMap, BTreeSet};

pub struct Delta {
    pub baseline: usize,
    pub expected: usize,
    pub reason: String,
    pub case: String,
}

pub fn parse(text: &str) -> Result<BTreeMap<String, Delta>, String> {
    let json: serde_json::Value =
        serde_json::from_str(text).map_err(|error| format!("expected deltas: {error}"))?;
    if json
        .get("schemaVersion")
        .and_then(serde_json::Value::as_u64)
        != Some(1)
    {
        return Err("expected deltas: `schemaVersion` must be 1".to_owned());
    }
    let entries = json
        .get("deltas")
        .and_then(serde_json::Value::as_array)
        .ok_or("expected deltas: no `deltas` array")?;
    entries
        .iter()
        .try_fold(BTreeMap::new(), |mut deltas, entry| {
            let text = |key: &str| {
                entry
                    .get(key)
                    .and_then(serde_json::Value::as_str)
                    .filter(|value| !value.is_empty() && !value.contains(['|', '\n', '\r']))
                    .map(str::to_owned)
                    .ok_or_else(|| {
                        format!(
                            "expected deltas: an entry has no one-line `{key}` without `|`: {entry}"
                        )
                    })
            };
            let count = |key: &str| {
                entry
                    .get(key)
                    .and_then(serde_json::Value::as_u64)
                    .and_then(|count| usize::try_from(count).ok())
                    .ok_or_else(|| {
                        format!("expected deltas: an entry has no `{key}` count: {entry}")
                    })
            };
            let rule = text("ruleId")?;
            let delta = Delta {
                baseline: count("baselineCount")?,
                expected: count("expectedCount")?,
                reason: text("reason")?,
                case: text("case")?,
            };
            match deltas.insert(rule.clone(), delta) {
                None => Ok(deltas),
                Some(_) => Err(format!("expected deltas: `{rule}` is listed twice")),
            }
        })
}

pub fn check_cases(deltas: &BTreeMap<String, Delta>, pack_files: &[&str]) -> Result<(), String> {
    deltas.iter().try_for_each(|(rule, delta)| {
        let (pack, name) = rule
            .split_once('/')
            .ok_or_else(|| format!("expected deltas: `{rule}` is not `<pack>/<rule>`"))?;
        let case_name = ["good", "bad"].iter().find_map(|kind| {
            delta
                .case
                .strip_prefix(&format!("{pack}/fixtures/{name}/{kind}/"))
        });
        let directory = format!("{}/", delta.case);
        match case_name {
            Some(case_name)
                if !case_name.contains('/')
                    && pack_files.iter().any(|file| file.starts_with(&directory)) =>
            {
                Ok(())
            }
            _ => Err(format!(
                "expected deltas: `{rule}` names `{}`, which is no bundled `{pack}/fixtures/{name}/<good|bad>/<case>` directory",
                delta.case
            )),
        }
    })
}

pub fn compare(
    baseline: &BTreeMap<String, usize>,
    candidate: &BTreeMap<String, usize>,
    deltas: &BTreeMap<String, Delta>,
) -> Result<(), String> {
    let rules: BTreeSet<&String> = baseline
        .keys()
        .chain(candidate.keys())
        .chain(deltas.keys())
        .collect();
    let problems: Vec<String> = rules
        .into_iter()
        .filter_map(|rule| {
            let was = baseline.get(rule).copied().unwrap_or(0);
            let now = candidate.get(rule).copied().unwrap_or(0);
            match deltas.get(rule) {
                None if was != now => Some(format!(
                    "`{rule}` has {now} findings against the baseline's {was}, and no expected delta lists it"
                )),
                Some(_) if was == now => Some(format!(
                    "`{rule}` equals the baseline ({now} findings), so its expected delta is stale; delete it"
                )),
                Some(delta) if (delta.baseline, delta.expected) != (was, now) => Some(format!(
                    "`{rule}` went {was} -> {now}, but its expected delta says {} -> {}",
                    delta.baseline, delta.expected
                )),
                _ => None,
            }
        })
        .collect();
    if problems.is_empty() {
        Ok(())
    } else {
        Err(problems.join("; "))
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::{Delta, check_cases, compare, parse};
    use std::collections::BTreeMap;

    fn counts(pairs: &[(&str, usize)]) -> BTreeMap<String, usize> {
        pairs
            .iter()
            .map(|(rule, count)| ((*rule).to_owned(), *count))
            .collect()
    }

    fn listed(rule: &str, baseline: usize, expected: usize) -> BTreeMap<String, Delta> {
        listed_case(rule, baseline, expected, "c")
    }

    fn listed_case(
        rule: &str,
        baseline: usize,
        expected: usize,
        case: &str,
    ) -> BTreeMap<String, Delta> {
        let delta = Delta {
            baseline,
            expected,
            reason: "r".to_owned(),
            case: case.to_owned(),
        };
        BTreeMap::from([(rule.to_owned(), delta)])
    }

    #[test]
    fn a_difference_passes_only_when_listed_with_both_counts() {
        let (was, now) = (counts(&[("a", 1), ("b", 3)]), counts(&[("a", 2), ("b", 3)]));
        assert!(compare(&was, &now, &listed("a", 1, 2)).is_ok());
        let unlisted = compare(&was, &now, &BTreeMap::new()).expect_err("unlisted");
        assert!(unlisted.contains("no expected delta"), "{unlisted}");
        let drifted = compare(&was, &now, &listed("a", 1, 3)).expect_err("drifted");
        assert!(drifted.contains("says 1 -> 3"), "{drifted}");
    }

    #[test]
    fn a_listed_rule_that_equals_the_baseline_is_stale() {
        let same = counts(&[("a", 1)]);
        let stale = compare(&same, &same, &listed("a", 1, 2)).expect_err("stale");
        assert!(stale.contains("stale"), "{stale}");
        let gone = compare(&same, &same, &listed("z", 0, 1)).expect_err("absent rule");
        assert!(
            gone.contains("`z` equals the baseline (0 findings)"),
            "{gone}"
        );
    }

    #[test]
    fn a_rule_absent_on_one_side_counts_as_zero() {
        let (was, now) = (counts(&[]), counts(&[("a", 2)]));
        assert!(compare(&was, &now, &listed("a", 0, 2)).is_ok());
    }

    #[test]
    fn parse_refuses_a_repeated_rule_a_wrong_schema_and_malformed_entries() {
        let entry = r#"{"ruleId":"a","baselineCount":1,"expectedCount":2,"reason":"r","case":"c"}"#;
        let twice = format!(r#"{{"schemaVersion":1,"deltas":[{entry},{entry}]}}"#);
        assert!(parse(&twice).is_err_and(|error| error.contains("listed twice")));
        assert!(parse(r#"{"schemaVersion":2,"deltas":[]}"#).is_err());
        let once = format!(r#"{{"schemaVersion":1,"deltas":[{entry}]}}"#);
        assert_eq!(parse(&once).map(|deltas| deltas.len()), Ok(1));
        for malformed in [
            entry.replace(r#","case":"c""#, ""),
            entry.replace(r#""reason":"r""#, r#""reason":"""#),
            entry.replace(r#""reason":"r""#, r#""reason":"a | b""#),
            entry.replace(r#""reason":"r""#, r#""reason":"a\nb""#),
            entry.replace(r#""baselineCount":1"#, r#""baselineCount":"1""#),
        ] {
            let text = format!(r#"{{"schemaVersion":1,"deltas":[{malformed}]}}"#);
            assert!(parse(&text).is_err(), "{text}");
        }
    }

    #[test]
    fn a_case_must_be_a_bundled_case_of_the_listed_rule() {
        let files = ["p/fixtures/r/bad/x/a.json", "p/fixtures/other/bad/y/a.json"];
        let check = |case: &str| check_cases(&listed_case("p/r", 1, 2, case), &files);
        assert!(check("p/fixtures/r/bad/x").is_ok());
        for wrong in [
            "p/fixtures/other/bad/y",
            "p/fixtures/r/bad/missing",
            "p/fixtures/r/bad",
            "p/fixtures/r/ugly/x",
        ] {
            assert!(check(wrong).is_err(), "{wrong}");
        }
    }

    #[test]
    fn the_shipped_expected_deltas_parse() {
        assert!(parse(include_str!("../expected-deltas.json")).is_ok());
    }
}
