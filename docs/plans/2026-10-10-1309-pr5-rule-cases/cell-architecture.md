## cell-architecture/service-exports-no-layer

### Clauses

Numbered from the ```grit block and the `locate` frontmatter (text quoted verbatim). `B*` = body (grit) branches, `L*` = `locate` branches.

**Multifile wrapper**
- **C1** `file($name, $body) where { $name <: r".*\.service\.ts", ... }` — the path filter (both body and `locate`, which has no filter of its own).
- **C2** `$body <: contains or { ... }` — `contains` over a five-branch `or`.

**Body branches**
- **C3** `` `Layer.$method($...)` ``
- **C4** `` `export const $live = $_` where { $live <: r".*Live" } ``
- **C5** `export_statement()` where { `$statement <: r"export\s+(?:(?:const|let|var)\s+[A-Za-z0-9_$]+\s*(?::[\s\S]+?)?=|default\b)\s*(?:[A-Za-z0-9_$]+\s*\.\s*)*(?:layer|[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)\s*(?:(?:as|satisfies)\b[\s\S]*)?;?"` }`
  - **C5a** `(?:const|let|var)` — the declaration keyword
  - **C5b** `[A-Za-z0-9_$]+` — the declared-name class
  - **C5c** `(?::[\s\S]+?)?=` — optional type annotation + mandatory `=`
  - **C5d** `default\b` — the default-export alternative
  - **C5e** `(?:[A-Za-z0-9_$]+\s*\.\s*)*` — member-access prefix
  - **C5f** `(?:layer|[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)` — the bound value's name (anchored, full match)
  - **C5g** `(?:(?:as|satisfies)\b[\s\S]*)?;?` — `as`/`satisfies` tail and the optional `;`
- **C6** `export_statement()` where { `$statement <: not r"export\s+type\b[\s\S]*"`, `$statement <: contains export_specifier(name=$exported) as $specifier where { $specifier <: not r"type\s[\s\S]*"`, `$exported <: r"(?:layer|[A-Za-z0-9_$]*Layer|[A-Za-z0-9_$]*Live)" } }`
  - **C6a** statement is not `export type …`
  - **C6b** `contains export_specifier(name=$exported)` — the specifier must exist and be bound by `name=`
  - **C6c** the specifier is not `type …`
  - **C6d** the exported name is `layer` / `*Layer` / `*Live`
- **C7** `public_field_definition(name=$field)` where { `$field <: r"(?:layer|[A-Za-z]*Layer)"` }

**Locate branches**
- **L1** `` `Layer.$method($...)` ``
- **L2** `` `export const $live = $_` `` where { `$live <: r".*Live"` }
- **L3** the C5 regex over `export_statement()`
- **L4** `export_specifier(name=$exported) as $specifier` where { `$specifier <: not r"type\s[\s\S]*"`, `$specifier <: not within export_statement() as $type_only where { $type_only <: r"export\s+type\b[\s\S]*" }`, `$exported <: r"(?:layer|…*Layer|…*Live)"` }
  - **L4a** specifier is not `type …` (locate-local form)
  - **L4b** specifier is not inside an `export type …` statement (locate-local; the body uses C6a instead)
  - **L4c** the exported name
- **L5** `public_field_definition(name=$field)` where { `$field <: r"(?:layer|[A-Za-z]*Layer)"` }

### Cases

**good (8)**
- `good/port-only` — clean `*.service.ts` with `Effect.flatMap(Clock.asEffect(), …)` and `export const now = …`.
- `good/layer-type-alias` — `export type StageLayer = Layer.Layer<Clock>`: a `Layer.` *type reference* (no call) and an `export type` alias.
- `good/layer-in-driver` — the Layer lives in `packages/app/src/drivers/clock.ts`, not in a `*.service.ts`.
- `good/live-in-sibling-module` — the Layer lives in `packages/app/src/clock.layer.ts`.
- `good/annotated-value-export` — `export const delayMs: number = defaultDelay`, `export const layerLabel = formatTime satisfies (…)`, `export default formatTime`.
- `good/layer-prefixed-field` — `static readonly layerCount = 3`: `layer` must be anchored, not a prefix.
- `good/reexported-driver-value` — `export { formatTime, layerCount } from './drivers/clock.js'` in a service module, Layer in the driver.
- `good/type-reexport` — `export type { ClockLayer } from …` and `export { type ClockLive } from …`.

**bad (14)** — one line each
- `aliased-static-field` — `static readonly captureLayer = consoleLayer` → L5 with `*Layer`.
- `exported-layer-without-tag` — `export const layer = Layer.succeed(…)` → L1 at the call, **not** at the statement.
- `layer-beside-clean-port` — a clean `clock.service.ts` beside a dirty `timer.service.ts` whose `export const TimerLive = Layer.succeed(…)` is located at the whole statement (6:1-6:72).
- `layer-factory` — `export const sessionLayer = (id: string) => Layer.effect(…)` → L1 at 7:45.
- `live-alias` — `export const ProbeLive = probeLayer` → whole statement 6:1-6:36.
- `reexported-driver-layer` — `export { layer as clockLayer } from …` → the specifier 6:10-6:29.
- `reexported-live` — `export { ClockLive } from …` → the specifier 6:10-6:19.
- `static-layer-member` — `static readonly layer: Layer.Layer<Clock> = Layer.succeed(…)` → field 10:3-10:105, left of the inner call.
- `annotated-export-binding` — `export const clock: Layer.Layer<Clock> = ClockDriver.layer` → C5c.
- `asserted-export-binding` — `export const clock = layer as Layer.Layer<Clock>` → `as` tail.
- `satisfies-export-binding` — `… satisfies Layer.Layer<Clock>` → `satisfies` tail.
- `default-export-layer` — `export default layer` → C5d.
- `export-bound-to-driver-member` — `export const clock = ClockDriver.layer` → C5e.
- `export-bound-to-imported-layer` — `export const gitDiff = layer` → C5f `layer` arm.

(Total 22.)

### Coverage

Clause → the existing case that fails under each plausible one-step edit. Anchoring evidence: `good/layer-prefixed-field` (`layerCount`) and `good/reexported-driver-value` (`layerCount`) only stay clean because `<:` is an anchored full match, so those two cases are what pin anchoring for C7/L5 and C6d/L4c.

- **C1** — *widened to any `.ts`* → `good/layer-in-driver`, `good/live-in-sibling-module`, `good/reexported-driver-value` (each has a driver/sibling module holding a Layer). *widened to include `*.layer.ts`* → `good/live-in-sibling-module`. *`.*` dropped* → every bad case. *escape loss `\.` → `.`* → **GAP** (G1): no fixture has a file whose name matches `.service.ts` with a non-literal dot (e.g. `…/service.ts`).
- **C2** — *`contains` dropped / `or` → `and`* → every bad case (each matches exactly one branch).
- **C3** — *namespace widened to `$X.$method($...)`* → `good/port-only` (`Effect.flatMap`, `Clock.asEffect()`). *narrowed to `Layer.succeed`* → `bad/layer-factory` (`Layer.effect`). *`($...)` dropped* or *`Layer.$method` → `Layer`* → `good/layer-type-alias` (`Layer.Layer<Clock>`), `good/layer-in-driver`'s driver file.
- **C4/L2** — *`.*Live` widened to `.*`* → `good/port-only` (`now`), `good/annotated-value-export` (`delayMs`, `layerLabel`). *branch or `$_` narrowed to a call* → `bad/layer-beside-clean-port` (its `6:1-6:72` whole-statement range exists only via L2; without L2 the range becomes the `Layer.succeed` call at 6:26). *`export const` narrowed to `let`* → same case.
- **C5a** — `const` only → all bad `export const` cases. `let` dropped → **GAP** (G2). `var` dropped → **GAP** (G3).
- **C5b** — class narrowed to `[A-Za-z]+` → **GAP** (G4): no exported binding name contains a digit, `_` or `$`.
- **C5c** — annotation group dropped → `bad/annotated-export-binding`; made mandatory → `bad/export-bound-to-imported-layer` / `bad/export-bound-to-driver-member`. Multi-line annotation (`[\s\S]` → `.`) → **GAP** (G8).
- **C5d** — alternative dropped → `bad/default-export-layer`; alternative widened (any `export default`) → `good/annotated-value-export` (`export default formatTime`).
- **C5e** — group dropped entirely → `bad/export-bound-to-driver-member`. `*` → single segment, or `\s*\.\s*` → `\.` → **GAP** (G5): no fixture has a nested member or whitespace around `.`.
- **C5f** — `layer` arm dropped → `bad/export-bound-to-imported-layer`, `bad/default-export-layer`. `*Live` arm dropped → no killing case (see Redundant). `*Layer` arm dropped → no case has a service binding bound to `X…Layer`; the class digit/underscore part → **GAP** (G4). Anchoring dropped (`.*[Ll]ayer`) → **GAP** (G6).
- **C5g** — `as` dropped → `bad/asserted-export-binding`; `satisfies` dropped → `bad/satisfies-export-binding`; tail made mandatory → `bad/export-bound-to-driver-member`. `;?` dropped → **GAP** (G7): no fixture ends an export statement with `;`. Multi-line tail → **GAP** (G8).
- **C6a** — guard dropped → `good/type-reexport` line 6.
- **C6b** — `contains` dropped → `bad/reexported-driver-layer`; `name=` → `alias=` → `bad/reexported-live` (no alias to bind).
- **C6c** — guard dropped → `good/type-reexport` line 7.
- **C6d/L4c** — `layer` arm → `bad/reexported-driver-layer`; `*Live` arm → `bad/reexported-live`; anchoring → `good/reexported-driver-value` (`layerCount`). `*Layer` arm, and digits in the class → **GAP** (G9).
- **L4a / L4b** — **GAP** (G10): no *bad* fixture contains a type-only re-export, so the locate-local type guards are never exercised; `good/type-reexport` never runs `locate` because the body does not match.
- **L1** — dropped → `bad/exported-layer-without-tag` (4:22-4:75) and `bad/layer-factory` (7:45-7:102) would lose their ranges.
- **L3** — dropped → `bad/annotated-export-binding`, `bad/default-export-layer`, `bad/export-bound-to-driver-member` (their ranges are the whole statement).
- **L5** — dropped → `bad/static-layer-member`, `bad/aliased-static-field`. `public_field_definition` widened to any field (private included) → **GAP** (G12). Name class `[A-Za-z]*` widened to `[A-Za-z0-9_$]*` → **GAP** (G11). `layer`→`layer\w*` widening → `good/layer-prefixed-field`.
- **One finding per directory / leftmost** — `bad/layer-beside-clean-port` (clean sibling file not reported) and `bad/static-layer-member` (field at 10:3 beats the inner `Layer.succeed` at a later column).

### Proposed cases

Range convention confirmed against existing fixtures: end column is exclusive (`bad/aliased-static-field` reports `5:3-5:46` on a 45-character line). Each file below starts with the shown contents at the shown path.

**G1 — good, `good/driver-named-service`** (C1 escape loss `\.` → `.`)
`packages/app/src/drivers/service.ts`:
```ts
import * as Layer from 'effect/Layer'

import { Clock } from '../clock.service.js'

export const layer: Layer.Layer<Clock> = Layer.succeed(Clock, Clock.of({ now: 0 }))
```
No `expected-locations.txt`. With `.*.service.ts` (unescaped) this path matches (the `.` before `service` binds to `/`), the rule reports, and the good case fails. With `.*\.service\.ts` it does not match — verified.

**G2 — bad, `bad/export-let-binding`** (C5a `let`)
`packages/app/src/clock.service.ts`:
```ts
import * as Context from 'effect/Context'
import * as ClockDriver from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {}

export let clock = ClockDriver.layer
```
`expected-locations.txt`:
```
cell-architecture/service-exports-no-layer packages/app/src/clock.service.ts:7:1-7:37
```

**G3 — bad, `bad/export-var-binding`** (C5a `var`) — same file with `export var clock = ClockDriver.layer` (36 chars), same expected line:
```
cell-architecture/service-exports-no-layer packages/app/src/clock.service.ts:7:1-7:37
```

**G4 — bad, `bad/underscored-binding-name`** (C5b and C5f classes)
`packages/app/src/clock.service.ts`:
```ts
import * as Context from 'effect/Context'
import { driver_2Layer } from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {}

export const clock_2 = driver_2Layer
```
`expected-locations.txt`:
```
cell-architecture/service-exports-no-layer packages/app/src/clock.service.ts:7:1-7:37
```
Kills narrowing the declared-name class to `[A-Za-z]+` (C5b) and narrowing the bound-name class to `[A-Za-z]*Layer` (C5f). Verified full match on the current regex.

**G5 — bad, `bad/export-bound-to-spaced-nested-member`** (C5e)
`packages/app/src/clock.service.ts`:
```ts
import * as Context from 'effect/Context'
import * as ClockDriver from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {}

export const clock = ClockDriver . inner . layer
```
`expected-locations.txt`:
```
cell-architecture/service-exports-no-layer packages/app/src/clock.service.ts:7:1-7:49
```
Kills `*` → one segment and `\s*\.\s*` → `\.` in a single statement (48 chars, verified full match).

**G6 — good, `good/layer-builder-binding`** (C5f anchoring)
`packages/app/src/clock.service.ts`:
```ts
import * as Context from 'effect/Context'
import { layerBuilder } from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {}

export const clock = layerBuilder
```
No `expected-locations.txt`. Today no branch matches. Under either unanchored slip (`layer` → `.*layer`, `[A-Za-z0-9_$]*Layer` → `.*[Ll]ayer`) L3 matches, the finding is placed, and this good case fails. Note `bad/aliased-static-field` pins the same anchoring for L5 and `good/reexported-driver-value` for L4c; C5f's anchoring has no equivalent.

**G7 — bad, `bad/semicolon-export-binding`** (C5g `;?`)
`packages/app/src/clock.service.ts`:
```ts
import * as Context from 'effect/Context'
import * as ClockDriver from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {}

export const clock = ClockDriver.layer;
```
`expected-locations.txt`:
```
cell-architecture/service-exports-no-layer packages/app/src/clock.service.ts:7:1-7:40
```
Range assumes the semicolon is part of the `export_statement` node (standard tree-sitter-typescript); if it is not, this becomes `7:1-7:39`. UNSURE on that one column.

**G8 — bad, `bad/multiline-annotated-binding`** (C5c/C5g `[\s\S]`)
`packages/app/src/clock.service.ts`:
```ts
import type * as Layer from 'effect/Layer'
import * as Context from 'effect/Context'
import * as ClockDriver from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {}

export const clock:
  Layer.Layer<Clock> = ClockDriver.layer satisfies {
    readonly tag: 'app/Clock'
  }
```
`expected-locations.txt`:
```
cell-architecture/service-exports-no-layer packages/app/src/clock.service.ts:7:1-10:4
```
UNSURE on one point: this kills a `[\s\S]` → `.` slip only if `.` does not match a newline in this engine's regex flavour. It unambiguously pins that a multi-line annotation and a multi-line `satisfies` type are accepted at all, which no fixture currently checks.

**G9 — bad, `bad/reexported-layered-name`** (C6d/L4c `*Layer` arm and the digit class)
`packages/app/src/clock.service.ts`:
```ts
import * as Context from 'effect/Context'
import type * as Effect from 'effect/Effect'

export class Clock extends Context.Service<Clock, { readonly now: Effect.Effect<number> }>()('app/Clock') {}

export { Clock2Layer } from './drivers/clock.js'
```
`expected-locations.txt`:
```
cell-architecture/service-exports-no-layer packages/app/src/clock.service.ts:6:10-6:21
```
Kills dropping the `*Layer` alternative from the specifier name regex and narrowing `[A-Za-z0-9_$]` to `[A-Za-z]`.

**G10 — bad, `bad/type-reexport-before-violation`** (L4a and L4b)
`packages/app/src/clock.service.ts`:
```ts
import * as Context from 'effect/Context'
import * as Layer from 'effect/Layer'

export { type ClockLive } from './drivers/clock.js'

export type { ClockLayer } from './drivers/clock.js'

export const clock = Layer.succeed(Clock, Clock.of({ now: 0 }))
```
`expected-locations.txt`:
```
cell-architecture/service-exports-no-layer packages/app/src/clock.service.ts:8:22-8:64
```
The body matches only through C3 (`Layer.succeed`), so the file is scanned and `locate` runs. Removing L4a places the finding at 4:10-4:24, removing L4b at 6:15-6:25; either way the expected line fails. The `good/type-reexport` case cannot do this because its body never matches.

**G11 — good, `good/underscored-layer-field`** (C7/L5 `[A-Za-z]*` vs `[A-Za-z0-9_$]*`)
`packages/app/src/stack.service.ts`:
```ts
import * as Context from 'effect/Context'
import { driverLayer } from './drivers/clock.js'

export class Stack extends Context.Service<Stack, { readonly depth: number }>()('app/Stack') {
  static readonly driver_2Layer = driverLayer
}
```
No `expected-locations.txt`. Today nothing matches. If the field regex is aligned with C5f/C6d to `[A-Za-z0-9_$]*Layer`, this reports and the case fails. **Design flag, not just a gap:** the three name regexes are inconsistent — C5f and C6d/L4c admit digits/`_`/`$` before `Layer`, C7/L5 does not, while the README promises "a class field named `layer` or `*Layer`". This case pins the narrower behaviour; if the wider one is intended, the rule must change and this case becomes a bad one.

**G12 — good, `good/private-layer-field`** (`public_field_definition`)
`packages/app/src/clock.service.ts`:
```ts
import * as Context from 'effect/Context'
import { driverLayer } from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {
  private readonly layer = driverLayer
}
```
No `expected-locations.txt`. Widening `public_field_definition` to any field definition reports at 4:3-4:39 and the good case fails.

### Redundant cases

- **`bad/live-alias`** (`export const ProbeLive = probeLayer`, expects `6:1-6:36`) — no plausible single-step bug makes it fail while the rest of the pack still passes. Its expected range is reachable through L3 alone: the regex full-matches the statement via C5f's `*Layer` arm on `probeLayer` (verified), which yields the identical `6:1-6:36`. The only thing unique to it is L2's `.*Live` on the *declared* name, and `bad/layer-beside-clean-port` already pins that path with a range that only L2 can produce. Deleting this case loses no mutation coverage; it is kept, if at all, as a second reading of "`ProbeLive` bound to `probeLayer`".
- Borderline but retained: `bad/static-layer-member` overlaps `bad/layer-beside-clean-port` on "leftmost wins", but it is the only case pinning that a `static readonly` field is a `public_field_definition` and that the field outranks an inner `Layer.` call in the same line.
- `good/layer-in-driver` and `good/live-in-sibling-module` look interchangeable for C1 (both put a Layer in a non-`.service.ts` file), but they are not redundant: only the `.layer.ts` one dies if C1 is widened to `.*\.(service|layer)\.ts`, and only the `src/drivers/` one dies if the rule were scoped to "a module named after a driver directory".

**1 rule, 12 gaps, 12 proposed cases**