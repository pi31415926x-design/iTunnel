<template>
  <div class="p-6 space-y-4">
    <!-- Header -->
    <div class="flex items-center justify-between">
      <h1 class="text-xl font-semibold">Todo list</h1>
    </div>

    <!-- Filters -->
    <div class="flex gap-2">
      <input
        class="border px-3 py-1 rounded w-64 dark:border-slate-700 bg-white dark:bg-slate-800 text-sm dark:text-gray-300"
        placeholder="Search message"
      />

      <select class="border px-2 py-1 rounded dark:border-slate-700 bg-white dark:bg-slate-800 text-sm dark:text-gray-300">
        <option>ALL</option>
        <option>INFO</option>
        <option>WARN</option>
        <option>ERROR</option>
      </select>
    </div>

    <!-- Log list -->
    <div class="border rounded overflow-hidden text-sm font-mono">
      <div
        v-for="log in logs"
        :key="log.id"
        class="grid grid-cols-[120px_80px_120px_1fr] gap-2 px-3 py-2 border-b last:border-b-0 dark:border-slate-700 bg-white dark:bg-slate-800 items-center"
      >
        <span class="text-slate-500">{{ log.time }}</span>
        <span :class="levelClass(log.level)">
          {{ log.level }}
        </span>
        <span class="text-slate-600">{{ log.module }}</span>
        <span>{{ log.message }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
const logs = [
  {
    id: 1,
    time: "2025/12/24",
    level: "ERROR",
    module: "UI",
    message: "移动端的UI没有解决好, sidebar会挡住content",
  },
  {
    id: 2,
    time: "10:22:17",
    level: "WARN",
    module: "api",
    message: "request timeout",
  },
  {
    id: 3,
    time: "10:23:41",
    level: "ERROR",
    module: "db",
    message: "connection failed",
  },
]

const levelClass = (level: string) => {
  switch (level) {
    case "ERROR":
      return "text-red-600 font-semibold"
    case "WARN":
      return "text-amber-600"
    default:
      return "text-slate-600"
  }
}
</script>
