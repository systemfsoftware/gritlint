---
"@systemfsoftware/gritlint": minor
---

Each reported violation now points at the offending node, every bundled rule names a reason code and a next action, and `--format json` is a versioned document. New formats: `concise`, `github` and `github-summary`. To upgrade:

1. Open each of your rule files with a `---` block declaring `reasonCode` (kebab-case) and `nextAction`; `locate` is optional and no other key is allowed, so drop any `title:` line. Otherwise the run exits 2.
2. Read JSON as `{ "schemaVersion": 1, ... }`: `rule` is now `ruleId`, `path` and `line` move into `location` (`path`, plus an optional `range`), and `files_selected` is now `filesSelected`. `gritlint schema --report` prints the schema.
3. In every bad fixture case, commit `expected-locations.txt`: one line per reported violation, `<ruleId> <path>[:<line>:<column>-<line>:<column>]`.
