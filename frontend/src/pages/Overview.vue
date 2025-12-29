<template>
  <div class="p-6">
    <div class="grid grid-cols-12 gap-4">
      <StatCard title="Users" value="1,234" class="col-span-3" />
      <StatCard title="Revenue" value="$9,876" class="col-span-3" />
      <StatCard title="Errors" value="12" class="col-span-3" />
      <StatCard title="Latency" value="240ms" class="col-span-3" />

      <div class="col-span-12 rounded-lg border p-4 dark:border-slate-700 bg-white dark:bg-slate-800">
        <div class="text-sm text-gray-500 dark:text-gray-400 mb-2">Activity</div>
        <div class="h-40 flex items-center justify-center text-gray-400">
          Chart / Table Placeholder
        </div>
      </div>
    </div>
    
    <div class="p-4 text-sm text-slate-600">
      DARK MODE TEST
    </div>
  </div>
</template>

<script setup lang="ts">
import StatCard from "../components/StatCard.vue";
</script>
