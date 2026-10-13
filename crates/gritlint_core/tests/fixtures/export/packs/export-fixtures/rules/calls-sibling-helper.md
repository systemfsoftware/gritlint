---
reasonCode: calls-sibling-helper
nextAction: Inline the pattern.
---

# A rule calls a pattern another rule defines

A test-only rule for the Biome export.

```grit
language json
`"license": $value` where { $value <: unlicensed_text() }
```
