import { defineConfig } from 'vite'

export default defineConfig({
  resolve: { conditions: ['@systemfsoftware/source'] },
  ssr: { resolve: { conditions: ['@other/source'] } },
})
