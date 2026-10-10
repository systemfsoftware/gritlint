import * as Context from 'effect/Context'
import * as ClockDriver from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {}

export var clock = ClockDriver.layer
