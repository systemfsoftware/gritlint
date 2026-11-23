---
"@systemfsoftware/gritlint": minor
---

gritlint gains path rules, which judge file paths and never read a file.

- A rule whose fenced block is `path` takes `scope <regex>` (at most once) and `allow <regex>` lines. A path in scope that matches no `allow` pattern is one finding at that path. Patterns match the whole relative path, written with `/` separators.
- `gritlint check` now walks every non-ignored file whatever its extension, so a `.txt` file or a file with no extension is judged by a path rule. A file no rule language parses is selected as unparsed, is never read, and reaches no GritQL rule.
- A path rule takes no `locate` key and does not export to Biome (`export-path-only`).
- The files-selected count in a report now includes every walked file.
