# @systemfsoftware/gritlint@0.3.0

gritlint gains `gritlint lsp` and `gritlint export biome`, and `gritlint check` recognizes every multifile rule.

- `gritlint lsp` serves one error diagnostic per violation in each open document over the Language Server Protocol on stdio, from its unsaved text. It loads what `gritlint check` loads, refuses `initialize` with `check`'s text when `check` would refuse, and reports a file the engine cannot scan or read as a diagnostic.
- `gritlint export biome` lists which enabled rules export as Biome GritQL plugins; `--rule <ruleId>` prints one. Only single-file `css`, `json` and `js` rules export, so every bundled rule lists as `refused export-multifile`. Plugins are verified in gritlint's engine, not in Biome.
- `gritlint check` runs a rule whose `multifile` block follows a comment or definition per directory, like every multifile rule, instead of over every file of its language at once.
