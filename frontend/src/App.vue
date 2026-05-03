<template>
  <div class="
      min-h-screen
      bg-slate-50 dark:bg-slate-900
      text-slate-900 dark:text-slate-100
    ">
    <!-- Loading state -->
    <div v-if="!initialized" class="flex items-center justify-center h-screen">
      <div class="text-center">
        <div class="animate-spin rounded-full h-12 w-12 border-b-2 border-blue-500 mx-auto mb-4"></div>
        <p class="text-slate-600 dark:text-slate-400">Initializing application...</p>
      </div>
    </div>

    <!-- Main app: login is full-page; everything else uses dashboard chrome -->
    <router-view v-else v-slot="{ Component, route }">
      <component v-if="route.meta.public" :is="Component" />
      <DashboardLayout v-else>
        <component :is="Component" />
      </DashboardLayout>
    </router-view>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import DashboardLayout from "./layouts/Layout.vue";
import { useWireGuardMode } from './composables/useWireGuardMode';
import { useWireGuardStore } from './stores/wireguard';

const { initializeApp } = useWireGuardMode();
const wireguardStore = useWireGuardStore();
const router = useRouter();
const initialized = ref(false);

onMounted(async () => {
  try {
    await initializeApp();
    if (wireguardStore.mode === 'server') {
      const res = await fetch('/api/auth/status', { credentials: 'include' });
      const j = await res.json();
      if (j.login_required && !j.authenticated && router.currentRoute.value.path !== '/login') {
        await router.replace('/login');
      } else if (j.login_required && j.authenticated && router.currentRoute.value.path === '/login') {
        await router.replace('/');
      }
    }
  } catch (err) {
    console.error('Failed to initialize app:', err);
  } finally {
    initialized.value = true;
  }
});
</script>
