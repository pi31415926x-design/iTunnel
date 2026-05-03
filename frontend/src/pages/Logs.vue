<template>
  <div
    class="h-full overflow-y-auto p-4 lg:p-6 text-slate-900 dark:text-slate-100 bg-slate-50/50 dark:bg-slate-950/50">
    <div class="max-w-6xl mx-auto space-y-6">
      <!-- Page header -->
      <div class="flex flex-col sm:flex-row sm:items-center sm:justify-between gap-4 shrink-0">
        <div class="flex items-center gap-3">
          <div
            class="p-2.5 rounded-xl bg-red-500/10 text-red-600 dark:text-red-400 flex items-center justify-center shrink-0">
            <Bars4Icon class="h-5 w-5 shrink-0" />
          </div>
          <div>
            <h1 class="text-xl font-black tracking-tight">Application logs</h1>
            <p class="text-sm text-slate-500 dark:text-slate-400 font-medium">Live tail of backend log lines</p>
          </div>
        </div>
        <div class="flex flex-wrap items-center gap-2 justify-end">
          <input v-model="searchQuery"
            class="border px-3 py-2 rounded-xl w-full sm:w-64 text-sm dark:border-slate-700 bg-white dark:bg-slate-800 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-red-500/30 shadow-sm"
            placeholder="Search logs…" />

          <select v-model="levelFilter"
            class="border px-3 py-2 rounded-xl text-sm dark:border-slate-700 bg-white dark:bg-slate-800 dark:text-gray-300 focus:outline-none focus:ring-2 focus:ring-red-500/30 shadow-sm shrink-0">
            <option value="ALL">All levels</option>
            <option value="INFO">INFO</option>
            <option value="WARN">WARN</option>
            <option value="ERROR">ERROR</option>
            <option value="DEBUG">DEBUG</option>
            <option value="TRACE">TRACE</option>
          </select>

          <button type="button"
            class="inline-flex items-center justify-center px-4 py-2 rounded-xl text-sm font-semibold border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 text-slate-700 dark:text-slate-200 hover:bg-slate-50 dark:hover:bg-slate-700 focus:outline-none focus:ring-2 focus:ring-red-500/30 shadow-sm shrink-0 disabled:opacity-50"
            :disabled="clearingLogs" @click="clearLogs">
            {{ clearingLogs ? '…' : 'Clear log' }}
          </button>
        </div>
      </div>

      <!-- Log table card (table body/header compact only) -->
      <div
        class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl overflow-hidden shadow-lg shadow-slate-200/50 dark:shadow-none flex flex-col min-h-[12rem]">
        <div
          class="flex bg-slate-50 dark:bg-slate-800/80 border-b border-slate-200 dark:border-slate-800 text-[9px] leading-none font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wide shrink-0 select-none">
          <div class="px-1 py-1 relative border-r border-slate-200 dark:border-slate-800 flex items-center min-h-[1.35rem]"
            :style="{ width: colWidths.ts + 'px' }">
            Timestamp
            <div class="resize-handle" @mousedown="startResize('ts', $event)"></div>
          </div>
          <div class="px-1 py-1 relative border-r border-slate-200 dark:border-slate-800 flex items-center min-h-[1.35rem]"
            :style="{ width: colWidths.level + 'px' }">
            Level
            <div class="resize-handle" @mousedown="startResize('level', $event)"></div>
          </div>
          <div class="px-1 py-1 relative border-r border-slate-200 dark:border-slate-800 flex items-center min-h-[1.35rem]"
            :style="{ width: colWidths.target + 'px' }">
            Module
            <div class="resize-handle" @mousedown="startResize('target', $event)"></div>
          </div>
          <div class="px-1 py-1 flex-1 flex items-center min-h-[1.35rem]">Message</div>
        </div>

        <div class="overflow-y-auto flex-1 max-h-[calc(100vh-14rem)] p-0 scrollbar-thin scrollbar-thumb-gray-300 dark:scrollbar-thumb-slate-600">
          <div v-if="filteredLogs.length === 0" class="p-8 text-center text-slate-400 dark:text-slate-500 text-sm">
            Waiting for logs…
          </div>
          <div v-for="(log, index) in filteredLogs" :key="index"
            class="flex border-b border-slate-100 dark:border-slate-800/80 last:border-b-0 hover:bg-slate-50/80 dark:hover:bg-slate-800/40 items-start text-[11px] leading-tight font-mono transition-colors">
            <span class="px-1 py-0.5 text-slate-500 dark:text-slate-500 shrink-0 border-r border-slate-100 dark:border-slate-800/80"
              :style="{ width: colWidths.ts + 'px' }">{{ log.ts }}</span>
            <span class="px-1 py-0.5 border-r border-slate-100 dark:border-slate-800/80 shrink-0 whitespace-nowrap"
              :style="{ width: colWidths.level + 'px' }" :class="levelClass(log.level)">
              {{ log.level }}
            </span>
            <span
              class="px-1 py-0.5 text-slate-600 dark:text-slate-400 border-r border-slate-100 dark:border-slate-800/80 shrink-0 truncate"
              :style="{ width: colWidths.target + 'px' }" :title="log.target">
              {{ log.target }}
            </span>
            <span class="px-1 py-0.5 text-slate-800 dark:text-slate-300 flex-1 break-words whitespace-pre-wrap">{{
              log.message }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, reactive } from 'vue';
import { Bars4Icon } from '@heroicons/vue/24/outline';
import { serverFetch } from '@/server-fetch';

interface LogEntry {
  ts: string;
  level: string;
  target: string;
  message: string;
}

const logs = ref<LogEntry[]>([]);
const searchQuery = ref('');
const levelFilter = ref('ALL');
const clearingLogs = ref(false);
let intervalId: ReturnType<typeof setInterval> | undefined;

const colWidths = reactive({
  ts: 132,
  level: 52,
  target: 100
});

const startResize = (col: keyof typeof colWidths, event: MouseEvent) => {
  const startX = event.clientX;
  const startWidth = colWidths[col];

  const onMouseMove = (moveEvent: MouseEvent) => {
    const diff = moveEvent.clientX - startX;
    colWidths[col] = Math.max(50, startWidth + diff);
  };

  const onMouseUp = () => {
    document.removeEventListener('mousemove', onMouseMove);
    document.removeEventListener('mouseup', onMouseUp);
    document.body.style.cursor = '';
  };

  document.addEventListener('mousemove', onMouseMove);
  document.addEventListener('mouseup', onMouseUp);
  document.body.style.cursor = 'col-resize';
};

const fetchLogs = async () => {
  try {
    const res = await serverFetch('/api/logs');
    if (res.ok) {
      const data: LogEntry[] = await res.json();
      logs.value = data.reverse();
    }
  } catch (e) {
    console.error("Failed to fetch logs:", e);
  }
};

const filteredLogs = computed(() => {
  return logs.value.filter(log => {
    const matchesSearch = log.message.toLowerCase().includes(searchQuery.value.toLowerCase()) ||
      log.target.toLowerCase().includes(searchQuery.value.toLowerCase());
    const matchesLevel = levelFilter.value === 'ALL' || log.level === levelFilter.value;
    return matchesSearch && matchesLevel;
  });
});

const levelClass = (level: string) => {
  switch (level) {
    case "ERROR": return "text-red-600 dark:text-red-400 font-semibold";
    case "WARN": return "text-amber-600 dark:text-amber-400 font-semibold";
    case "INFO": return "text-emerald-600 dark:text-emerald-400 font-semibold";
    case "DEBUG": return "text-blue-600 dark:text-blue-400";
    case "TRACE": return "text-purple-600 dark:text-purple-400";
    default: return "text-slate-600 dark:text-slate-400";
  }
};

const clearLogs = async () => {
  if (clearingLogs.value) return;
  clearingLogs.value = true;
  try {
    const res = await serverFetch('/api/logs/clear', { method: 'POST' });
    if (res.ok) {
      logs.value = [];
    }
  } catch (e) {
    console.error('Failed to clear logs:', e);
  } finally {
    clearingLogs.value = false;
  }
};

onMounted(() => {
  fetchLogs();
  intervalId = setInterval(fetchLogs, 1000);
});

onUnmounted(() => {
  if (intervalId) clearInterval(intervalId);
});
</script>

<style scoped>
.resize-handle {
  position: absolute;
  right: 0;
  top: 0;
  bottom: 0;
  width: 4px;
  cursor: col-resize;
  background-color: transparent;
}

.resize-handle:hover {
  background-color: rgba(148, 163, 184, 0.45);
}
</style>
