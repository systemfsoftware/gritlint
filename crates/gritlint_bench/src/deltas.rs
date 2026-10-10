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
                    .filter(|value| !value.is_empty())
                    .map(str::to_owned)
                    .ok_or_else(|| format!("expected deltas: an entry has no `{key}`: {entry}"))
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
    use super::{Delta, compare, parse};
    use std::collections::BTreeMap;

    fn counts(pairs: &[(&str, usize)]) -> BTreeMap<String, usize> {
        pairs
            .iter()
            .map(|(rule, count)| ((*rule).to_owned(), *count))
            .collect()
    }

    fn listed(rule: &str, baseline: usize, expected: usize) -> BTreeMap<String, Delta> {
        let delta = Delta {
            baseline,
            expected,
            reason: "r".to_owned(),
            case: "c".to_owned(),
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
    fn parse_refuses_a_repeated_rule_and_a_wrong_schema() {
        let entry = r#"{"ruleId":"a","baselineCount":1,"expectedCount":2,"reason":"r","case":"c"}"#;
        let twice = format!(r#"{{"schemaVersion":1,"deltas":[{entry},{entry}]}}"#);
        assert!(parse(&twice).is_err_and(|error| error.contains("listed twice")));
        assert!(parse(r#"{"schemaVersion":2,"deltas":[]}"#).is_err());
        let once = format!(r#"{{"schemaVersion":1,"deltas":[{entry}]}}"#);
        assert_eq!(parse(&once).map(|deltas| deltas.len()), Ok(1));
    }
}
