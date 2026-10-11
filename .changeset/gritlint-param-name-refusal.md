---
"@systemfsoftware/gritlint": patch
---

When gritlint refuses a pack parameter name in its config, the error now gives the reason: the name is empty, or the name is not a GritQL identifier. Before this, every refused name, an empty one included, was reported as not being a GritQL identifier.
