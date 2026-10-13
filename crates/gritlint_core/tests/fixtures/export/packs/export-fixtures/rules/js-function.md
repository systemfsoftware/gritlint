---
reasonCode: js-function
nextAction: Use a pattern.
---

# A license is upper case

A test-only rule for the Biome export.

```grit
language json
function upper($text) js { return $text.text.toUpperCase(); }
`"license": $value` where { $shout = upper($value) }
```
