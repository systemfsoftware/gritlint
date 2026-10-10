import * as Context from 'effect/Context'
import { driver_2Layer } from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {}

export const clock_2 = driver_2Layer
