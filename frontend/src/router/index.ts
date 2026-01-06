import { createRouter, createWebHistory } from "vue-router"

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      component: () => import("../pages/Overview.vue"),
    },
    {
      path: "/logs",
      component: () => import("../pages/Logs.vue"),
    },
    {
      path: "/subscribe",
      component: () => import("../pages/Subscribe.vue"),
    },
    {
      path: "/settings",
      component: () => import("../pages/Settings.vue"),
    },
  ],
})
