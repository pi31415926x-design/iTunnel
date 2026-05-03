import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './index.css'
import App from './App.vue'
import { router } from "./router/index.ts"
import { useTheme } from "./components/useTheme.ts";
import faviconUrl from './assets/favicon.svg?url'

// Tab icon: use Vite-resolved URL so dev/build always match the served asset (plain /favicon.svg is brittle).
{
  let link = document.querySelector<HTMLLinkElement>('link[data-itunnel-icon="1"]')
  if (!link) {
    link = document.createElement('link')
    link.rel = 'icon'
    link.type = 'image/svg+xml'
    link.setAttribute('data-itunnel-icon', '1')
    document.head.appendChild(link)
  }
  link.href = faviconUrl
}

const app = createApp(App);
const pinia = createPinia();

// ⚠️ 初始化主题（只做一次）
useTheme().initTheme();

// 初始化状态管理和路由
app.use(pinia);
app.use(router);
app.mount("#app");
