import * as Context from 'effect/Context'
import * as Effect from 'effect/Effect'
import * as Layer from 'effect/Layer'

export interface ConsoleShape {
  readonly write: (line: string) => Effect.Effect<void>
}

export class Console extends Context.Service<Console, ConsoleShape>()('app/Console') {
  static readonly make = Effect.succeed(Console.of({ write: () => Effect.void }))
  static readonly layer = Layer.effect(this, this.make)
  static readonly layerTest = this.layer
}
