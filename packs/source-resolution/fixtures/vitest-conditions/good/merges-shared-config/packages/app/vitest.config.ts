import { mergeConfig } from 'vite'
import shared from '@systemfsoftware/vitest-config'

export default mergeConfig(shared, {
  ssr: { resolve: { conditions: ['@systemfsoftware/source'] } },
})
