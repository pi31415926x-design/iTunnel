<template>
  <div class="
      min-h-screen
      bg-slate-50 dark:bg-slate-900
      text-slate-900 dark:text-slate-100
    ">
    <DashboardLayout>
      <router-view />
    </DashboardLayout>
  </div>
</template>

<script setup lang="ts">
import DashboardLayout from "./layouts/Layout.vue";
</script>
