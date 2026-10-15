export default defineNuxtConfig({
  compatibilityDate: '2026-10-01',
  ssr: false,
  devtools: { enabled: false },
  devServer: { host: process.env.TAURI_DEV_HOST || 'localhost' },
  ignore: ['**/src-tauri/**', '**/plugins/**'],
  vite: {
    clearScreen: false,
    envPrefix: ['VITE_', 'TAURI_'],
    server: { strictPort: true },
  },
})
