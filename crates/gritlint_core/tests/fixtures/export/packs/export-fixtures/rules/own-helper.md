---
reasonCode: own-helper
nextAction: Choose a license.
---

# A rule defines the pattern it calls

A license is not `UNLICENSED`.

```grit
language json

pattern unlicensed_text() { `"UNLICENSED"` }

`"license": $value` where { $value <: unlicensed_text() }
```
