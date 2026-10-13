---
reasonCode: directory-read
nextAction: Nothing to do.
---

# Every JSON file in a directory is read together

A multifile rule that reads every JSON file of a directory and matches none.

```grit
language json
multifile {
  file($name, $body) where { $body <: contains `"never": $_` }
}
```
