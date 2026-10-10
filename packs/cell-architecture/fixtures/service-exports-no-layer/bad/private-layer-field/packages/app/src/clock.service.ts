import * as Context from 'effect/Context'
import { driverLayer } from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {
  private readonly layer = driverLayer
}
