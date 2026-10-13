---
reasonCode: flow-console
nextAction: Remove the call.
---

# Flow code does not log

A test-only rule for the Biome export.

```grit
language js(flow)
`console.log($message)`
```
