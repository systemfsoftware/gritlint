import * as Context from 'effect/Context'
import { layerBuilder } from './drivers/clock.js'

export class Clock extends Context.Service<Clock, { readonly now: number }>()('app/Clock') {}

export const clock = layerBuilder
