import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  server: {
    proxy: {
      '/setwg': 'http://127.0.0.1:8181',
      '/get_interfaces': 'http://127.0.0.1:8181',
      '/getwgstats': 'http://127.0.0.1:8181',
      '/logs': 'http://127.0.0.1:8181',
    }
  }
})
