import { defineConfig } from 'vite'

export default defineConfig({
  resolve: { conditions: ['@other/source'] },
  ssr: { resolve: { conditions: ['@systemfsoftware/source'] } },
})
