---
"@systemfsoftware/gritlint": patch
---

`source-resolution/tsconfig-condition` reports the tsconfig file that fails it. A project with no `customConditions` key was reported against the alphabetically first tsconfig in its directory, which could be a sibling that passes. The finding now points at the failing project's `include`. A project whose `customConditions` lacks the condition is still reported at that list.
