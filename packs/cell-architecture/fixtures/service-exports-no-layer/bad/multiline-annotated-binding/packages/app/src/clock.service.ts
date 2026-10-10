import type * as Layer from 'effect/Layer'
import * as Context from 'effect/Context'
import * as ClockDriver from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {}

export const clock:
  Layer.Layer<Clock> = ClockDriver.layer satisfies {
    readonly tag: 'app/Clock'
  }
