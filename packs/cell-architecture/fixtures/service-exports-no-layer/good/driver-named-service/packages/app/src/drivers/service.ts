import * as Layer from 'effect/Layer'

import { Clock } from '../clock.service.js'

export const layer: Layer.Layer<Clock> = Layer.succeed(Clock, Clock.of({ now: 0 }))
