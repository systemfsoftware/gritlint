---
reasonCode: commented-multifile
nextAction: Add a package.json.
---

# A multifile rule behind a comment

A test-only rule for the Biome export.

```grit
language json
// the comment comes before the multifile block
multifile {
  file($name, $body) where { $name <: r".*package\.json" }
}
```
