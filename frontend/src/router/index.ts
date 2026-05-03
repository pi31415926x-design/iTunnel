import { createRouter, createWebHistory } from "vue-router"

export const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: "/login",
      component: () => import("../pages/Login.vue"),
      meta: { public: true },
    },
    {
      path: "/",
      component: () => import("../pages/OverviewSwitcher.vue"),
    },
    {
      path: "/endpoints",
      component: () => import("../pages/Endpoints.vue"),
    },
    {
      path: "/peers",
      component: () => import("../pages/Peers.vue"),
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
