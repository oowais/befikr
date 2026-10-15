export default defineNuxtConfig({
  compatibilityDate: '2026-10-01',
  ssr: false,
  devtools: { enabled: false },
  css: ['@fontsource-variable/geist', '~/assets/main.css', '~/assets/glass.css'],
  app: {
    head: {
      title: 'Befikr',
      meta: [{ name: 'viewport', content: 'width=device-width, initial-scale=1, viewport-fit=cover' }],
      // Apply the saved design before first paint (no flash). See composables/useDesign.ts.
      script: [
        {
          innerHTML:
            "try{document.documentElement.dataset.design=localStorage.getItem('befikr.design')==='glass'?'glass':'classic'}catch(e){}",
        },
      ],
    },
  },
  devServer: { host: process.env.TAURI_DEV_HOST || 'localhost' },
  ignore: ['**/src-tauri/**', '**/plugins/**', '**/crates/**', '**/packages/**'],
  vite: {
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: { strictPort: true },
  },
})
