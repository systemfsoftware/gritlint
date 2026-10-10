import * as Context from 'effect/Context'
import type * as Effect from 'effect/Effect'

export class Clock extends Context.Service<Clock, { readonly now: Effect.Effect<number> }>()('app/Clock') {}

export { Clock2Layer } from './drivers/clock.js'
