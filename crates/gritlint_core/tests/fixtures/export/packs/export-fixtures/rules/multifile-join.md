---
reasonCode: multifile-join
nextAction: Add a package.json.
---

# A directory has a package.json

A test-only rule for the Biome export.

```grit
language json
multifile {
  file($name, $body) where { $name <: r".*package\.json" }
}
```
