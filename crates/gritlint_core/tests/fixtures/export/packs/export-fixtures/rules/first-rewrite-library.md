---
reasonCode: first-rewrite-library
nextAction: Fix it.
---

# A rewriting rule that calls the pack value refuses on the rewrite

A test-only rule for the Biome export.

```grit
language json
`"license": $value` => `"license": "MIT"` where { $value <: license_value() }
```
