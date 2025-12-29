<template>
  <div class="flex flex-1 flex-col min-w-0 rounded-lg border p-4 text-center dark:border-slate-700 bg-white dark:bg-slate-800">
    <div class="text-xs md:text-sm lg:text-base text-gray-500 dark:text-gray-400">
      {{ title }}
    </div>
    <div class="mt-2 text-sm md:text-2xl lg:text-3xl font-semibold text-gray-900 dark:text-gray-500">
      {{ value }}
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  title: string;
  value: string;
}>();
</script>
