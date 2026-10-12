---
reasonCode: first-language-function
nextAction: Fix it.
---

# A Flow rule with a function refuses on its language

A test-only rule for the Biome export.

```grit
language js(flow)
function upper($text) js { return $text.text.toUpperCase(); }
`console.log($message)` where { $shout = upper($message) }
```
