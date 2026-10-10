import * as Context from 'effect/Context'
import type { ConsoleLayer as Sink } from './stdout-console.js'
import { type ConsoleLayer as Pipe, formatLine as format } from './stdout-console.js'

export interface ConsoleShape {
  readonly sink: Sink
  readonly pipe: Pipe
  readonly format: typeof format
}

export class Console extends Context.Service<Console, ConsoleShape>()('app/Console') {}
