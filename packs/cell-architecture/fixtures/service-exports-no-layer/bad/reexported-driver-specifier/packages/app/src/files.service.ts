import * as Context from 'effect/Context'
import type * as Effect from 'effect/Effect'

export { readFile } from 'node:fs/promises'

export class Files extends Context.Service<Files, { readonly size: (path: string) => Effect.Effect<number> }>()(
  'app/Files',
) {}
