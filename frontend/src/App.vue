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

    <!-- Main app -->
    <DashboardLayout v-else>
      <router-view />
    </DashboardLayout>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref } from 'vue';
import DashboardLayout from "./layouts/Layout.vue";
import { useWireGuardMode } from './composables/useWireGuardMode';

const { initializeApp } = useWireGuardMode();
const initialized = ref(false);

onMounted(async () => {
  try {
    await initializeApp();
    initialized.value = true;
  } catch (err) {
    console.error('Failed to initialize app:', err);
    initialized.value = true;
  }
});
</script>
