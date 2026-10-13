---
reasonCode: first-multifile-yaml
nextAction: Fix it.
---

# A YAML multifile rule refuses as multifile

A test-only rule for the Biome export.

```grit
language yaml
multifile {
  file($name, $body) where { $name <: r".*\.yaml" }
}
```
