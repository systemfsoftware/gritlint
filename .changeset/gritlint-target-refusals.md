---
"@systemfsoftware/gritlint": minor
---

gritlint now parses a TypeScript object type or interface whose next member, on a new line, is a generic call signature (the shape Effect's `dual` overloads take), so such a module is judged instead of refusing the run. A target the engine cannot scan still refuses the run with exit 2, and stderr now carries one uncolored line with a reason code and a next action, the same in every format:

- `ERROR gritlint: <file>:<line>:<column>: target-does-not-parse: ...` when the file does not parse;
- `ERROR gritlint: <file>: target-too-large: ...` when the file is over the engine's 1,000,000-byte limit (before this, it was reported as a syntax error at 1:1);
- `ERROR gritlint: <file>: target-not-scanned: ...` for any other engine log on the file.

Stderr is colored only when it is a terminal. A consumer that matched the old `target file ... does not parse` message matches the reason code instead.
