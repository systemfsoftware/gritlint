import { defaultClientConditions, defaultServerConditions, defineConfig } from 'vite'

export default defineConfig({
  resolve: { conditions: [...defaultClientConditions, '@systemfsoftware/source'] },
  ssr: { resolve: { conditions: [...defaultServerConditions, '@systemfsoftware/source'] } },
})
