---
reasonCode: rewrite-license
nextAction: Let gritlint report it.
---

# A license is MIT

A test-only rule for the Biome export.

```grit
language json
`"license": $value` => `"license": "MIT"`
```
