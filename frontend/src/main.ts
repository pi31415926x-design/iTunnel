import { createApp } from 'vue'
import './index.css'
import App from './App.vue'
import { router } from "./router/index.ts"
import { useTheme } from "./components/useTheme.ts";
const app = createApp(App);
// ⚠️ 初始化主题（只做一次）
useTheme().initTheme();
app.use(router);
app.mount("#app");
