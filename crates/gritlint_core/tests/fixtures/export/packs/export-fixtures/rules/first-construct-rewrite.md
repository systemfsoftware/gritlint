---
reasonCode: first-construct-rewrite
nextAction: Fix it.
---

# A rewriting rule with a function refuses on the function

A test-only rule for the Biome export.

```grit
language json
function upper($text) js { return $text.text.toUpperCase(); }
`"license": $value` => `"license": "MIT"` where { $shout = upper($value) }
```
