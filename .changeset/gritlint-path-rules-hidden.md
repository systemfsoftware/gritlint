---
"@systemfsoftware/gritlint": patch
---

Path rules now judge hidden files, and a path rule no longer breaks a GritQL rule beside it.

- The walk reaches dot-named files and folders (`.DS_Store`, `.hidden/`), so a path rule judges them. No GritQL rule reads them, as before.
- A path rule's block was handed to every GritQL rule in the same scope as a library pattern, so a local path rule next to a local GritQL rule failed to compile. Path rules are no longer library patterns.
