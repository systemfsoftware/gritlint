---
reasonCode: calls-sibling
nextAction: Inline the pattern.
---

# A license calls the pack value

A test-only rule for the Biome export.

```grit
language json
`"license": $value` where { $value <: license_value() }
```
