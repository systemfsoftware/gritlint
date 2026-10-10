import * as Context from 'effect/Context'
import * as Layer from 'effect/Layer'

export { type ClockLive } from './drivers/clock.js'

export type { ClockLayer } from './drivers/clock.js'

export const clock = Layer.succeed(Clock, Clock.of({ now: 0 }))
