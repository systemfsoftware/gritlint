---
reasonCode: forbidden-license
nextAction: Choose a license the pack allows.
---

# A package does not use a "forbidden" license

A package does not use a "forbidden" license.
The pack names the license it forbids.

```grit
language json
`"license": $value` where { $value <: forbiddenLicense() }
```
