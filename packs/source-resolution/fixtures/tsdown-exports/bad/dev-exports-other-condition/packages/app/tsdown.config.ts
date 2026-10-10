export default defineConfig({
  exports: { devExports: '@other/source', customExports: injectTypes },
})
