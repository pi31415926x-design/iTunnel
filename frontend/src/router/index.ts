import { createRouter, createWebHistory } from "vue-router"

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/",
      component: () => import("../pages/ClientOverview.vue"),
    },
    {
      path: "/endpoints",
      component: () => import("../pages/Endpoints.vue"),
    },
    {
      path: "/logs",
      component: () => import("../pages/Logs.vue"),
    },
    {
      path: "/subscribe",
      component: () => import("../pages/Subscribe.vue"),
    },
  ],
})
