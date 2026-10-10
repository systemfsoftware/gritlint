import * as Context from 'effect/Context'
import * as Effect from 'effect/Effect'

export interface Retry {
  (times: number): <A, E, R>(effect: Effect.Effect<A, E, R>) => Effect.Effect<A, E, R>
  <A, E, R>(effect: Effect.Effect<A, E, R>, times: number): Effect.Effect<A, E, R>
}

export class RetryPolicy extends Context.Service<RetryPolicy, { readonly retry: Retry }>()('app/RetryPolicy') {}
