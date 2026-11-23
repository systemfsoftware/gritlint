---
reasonCode: tsconfig-dom-with-runtime-types
nextAction: Split the config by runtime: keep `DOM` in the browser role (tsconfig.app.json) and `node` or `@cloudflare/workers-types` in the runtime role (tsconfig.node.json).
locate: `"compilerOptions": $options` where { $options <: contains `"lib": $lib` where { $lib <: contains r"\"(?i:dom(?:\.[a-z]+)?|lib\.dom(?:\.[a-z]+)?\.d\.ts)\"" }, $options <: contains `"types": $types` where { $types <: contains r"\"(?:node|@types/node|@cloudflare/workers-types(?:/[^\"]*)?)\"" } }
---

# One config does not combine browser and server runtime types

A single tsconfig `compilerOptions` must not list `DOM` (or `DOM.Iterable`,
`DOM.AsyncIterable`) in `lib` and also list `node` or
`@cloudflare/workers-types` in `types`.

The browser and a Node or Worker runtime declare overlapping globals with
different types (`fetch`, `Response`, `setTimeout`, `URL`, `Request`). A config
that loads both type sets resolves each global to whichever declaration wins,
so code that is wrong in one runtime typechecks, and a server module can read
`window` without an error.

Only this one file is read: `lib` or `types` inherited through `extends` is not
resolved, so a combination that exists only after inheritance is not seen.

Fix it by giving each runtime its own role config: `DOM` in the browser role and
`node` or `@cloudflare/workers-types` in the runtime role.

```grit
language json

`"compilerOptions": $options` where {
  $options <: contains `"lib": $lib` where {
    $lib <: contains r"\"(?i:dom(?:\.[a-z]+)?|lib\.dom(?:\.[a-z]+)?\.d\.ts)\""
  },
  $options <: contains `"types": $types` where {
    $types <: contains r"\"(?:node|@types/node|@cloudflare/workers-types(?:/[^\"]*)?)\""
  }
}
```
