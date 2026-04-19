import { createApp } from 'vue'
import { createPinia } from 'pinia'
import './index.css'
import App from './App.vue'
import { router } from "./router/index.ts"
import { useTheme } from "./components/useTheme.ts";

const app = createApp(App);
const pinia = createPinia();

// ⚠️ 初始化主题（只做一次）
useTheme().initTheme();

// 初始化状态管理和路由
app.use(pinia);
app.use(router);
app.mount("#app");
