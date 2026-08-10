import { build } from 'vite'
import vue from '@vitejs/plugin-vue'

await build({
  root: process.cwd(),
  configFile: false,
  plugins: [vue()],
  build: {
    outDir: 'dist',
    emptyOutDir: true
  }
})
