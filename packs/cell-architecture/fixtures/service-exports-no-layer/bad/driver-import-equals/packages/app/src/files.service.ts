import * as Context from 'effect/Context'
import fs = require('node:fs/promises')

export class Files extends Context.Service<Files, { readonly read: typeof fs.readFile }>()('app/Files') {}
