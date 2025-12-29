## 项目日志

### 创建项目
```
1. npm create vite@latest dashboard -- --template vue-ts
2. npm install

安装 Tailwind CSS
npm install -D tailwindcss @tailwindcss/cli postcss autoprefixer
npx tailwindcss init -p
会生成
tailwind.config.js
postcss.config.js
```

### 配置 Tailwind 扫描路径
tailwind.config.js
```
export default {
  content: [
    "./index.html",
    "./src/**/*.{vue,js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {},
  },
  plugins: [],
}
```
- 注入 Tailwind 到 CSS
src/index.css
```
@tailwind base;
@tailwind components;
@tailwind utilities;

html {
  @apply text-gray-800;
}
```

- 确保 main.ts 引入 CSS
src/main.ts
```
import { createApp } from "vue";
import App from "./App.vue";
import "./index.css";

createApp(App).mount("#app");
```

