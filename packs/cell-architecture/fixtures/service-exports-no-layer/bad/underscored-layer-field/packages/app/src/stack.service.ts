import * as Context from 'effect/Context'
import { makeDriver } from './drivers/clock.js'

export class Stack extends Context.Service<Stack, { readonly depth: number }>()('app/Stack') {
  static readonly driver_2Layer = makeDriver()
}
