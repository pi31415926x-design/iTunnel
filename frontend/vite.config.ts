import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'
import { execSync } from 'child_process'

function getAppVersion(): string {
  let shortCommit = 'unknown'
  try {
    shortCommit = execSync('git rev-parse --short HEAD', {
      encoding: 'utf-8',
      stdio: ['ignore', 'pipe', 'ignore']
    }).trim()
  } catch {
    // Keep fallback commit id.
  }

  try {
    const latestTag = execSync('git describe --tags --abbrev=0', {
      encoding: 'utf-8',
      stdio: ['ignore', 'pipe', 'ignore']
    }).trim()
    if (latestTag && shortCommit) {
      return `${latestTag}-${shortCommit}`
    }
  } catch {
    // No tags yet; fall back to commit-based version.
  }
  return `untagged-${shortCommit}`
}

const appVersion = getAppVersion()

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  define: {
    __APP_VERSION__: JSON.stringify(appVersion)
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src')
    }
  },
  server: {
    proxy: {
      '/api': 'http://127.0.0.1:8181',
    }
  }
})
