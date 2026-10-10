---
"@systemfsoftware/gritlint": minor
---

Each reported violation now points at the offending node, every bundled rule names a reason code and a next action, and `--format json` is a versioned document. New formats: `concise`, `github` (workflow annotations) and `github-summary` (Markdown for the step summary). To upgrade:

1. Open each of your own rule files with a `---` block declaring `reasonCode` (kebab-case) and `nextAction` (one sentence); `locate` is optional. Without them the run exits 2, naming the file and the key.
2. Read JSON as `{ "schemaVersion": 1, ... }` with camelCase keys: `files_selected` is now `filesSelected`. `gritlint schema --report` prints its schema.
3. In every bad fixture case, commit `expected-locations.txt`: one line per reported violation, `<ruleId> <path>[:<line>:<column>-<line>:<column>]`.
