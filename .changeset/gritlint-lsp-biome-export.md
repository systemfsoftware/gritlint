---
"@systemfsoftware/gritlint": minor
---

gritlint gains two subcommands.

- `gritlint lsp` reports rule violations to an editor over the Language Server Protocol on stdio: one error diagnostic per violation in each open document, computed from its unsaved text, multifile rules included. It loads exactly what `gritlint check` loads from the directory it starts in (or ROOT), refuses `initialize` with `check`'s text when `check` would refuse, and reports a file the engine cannot scan as a diagnostic instead of stopping.
- `gritlint export biome` lists which enabled rules export as Biome GritQL plugins and, with `--rule <ruleId>`, prints one on stdout. Only single-file `css`, `json` and `js` rules export; every bundled rule is multifile and lists as `refused export-multifile`. Plugins are verified in gritlint's own engine, not in Biome.
