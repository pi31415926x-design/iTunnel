<template>
  <div class="h-full flex flex-col p-4 space-y-2">
    <!-- Header -->
    <div class="flex items-center justify-between shrink-0">
      <h1 class="text-lg font-semibold dark:text-gray-100">Application Logs</h1>
      <div class="flex gap-2">
        <input
          v-model="searchQuery"
          class="border px-2 py-1 rounded-md w-64 dark:border-slate-700 bg-white dark:bg-slate-800 text-xs dark:text-gray-300 focus:outline-none focus:ring-1 focus:ring-blue-500"
          placeholder="Search logs..."
        />

        <select
          v-model="levelFilter"
          class="border px-2 py-1 rounded-md dark:border-slate-700 bg-white dark:bg-slate-800 text-xs dark:text-gray-300 focus:outline-none focus:ring-1 focus:ring-blue-500"
        >
          <option value="ALL">ALL LEVELS</option>
          <option value="INFO">INFO</option>
          <option value="WARN">WARN</option>
          <option value="ERROR">ERROR</option>
          <option value="DEBUG">DEBUG</option>
          <option value="TRACE">TRACE</option>
        </select>
      </div>
    </div>

    <!-- Log list -->
    <div class="flex-1 border rounded-lg overflow-hidden bg-white dark:bg-slate-900 dark:border-slate-700 shadow-sm flex flex-col">
       <!-- Table Header -->
       <div 
         class="flex bg-gray-50 dark:bg-slate-800 border-b dark:border-slate-700 text-xs font-medium text-gray-500 dark:text-gray-400 uppercase tracking-wider shrink-0 select-none"
       >
         <div 
           class="px-2 py-1 relative border-r dark:border-slate-700 flex items-center" 
           :style="{ width: colWidths.ts + 'px' }"
         >
           Timestamp
           <div class="resize-handle" @mousedown="startResize('ts', $event)"></div>
         </div>
         <div 
           class="px-2 py-1 relative border-r dark:border-slate-700 flex items-center" 
           :style="{ width: colWidths.level + 'px' }"
         >
           Level
           <div class="resize-handle" @mousedown="startResize('level', $event)"></div>
         </div>
         <div 
           class="px-2 py-1 relative border-r dark:border-slate-700 flex items-center" 
           :style="{ width: colWidths.target + 'px' }"
         >
           Module
           <div class="resize-handle" @mousedown="startResize('target', $event)"></div>
         </div>
         <div class="px-2 py-1 flex-1">Message</div>
       </div>

       <!-- Table Body -->
       <div class="overflow-y-auto flex-1 p-0 scrollbar-thin scrollbar-thumb-gray-300 dark:scrollbar-thumb-slate-600">
         <div v-if="filteredLogs.length === 0" class="p-8 text-center text-gray-400 dark:text-gray-500 text-xs">
           Waiting for logs...
         </div>
         <div
           v-for="(log, index) in filteredLogs"
           :key="index"
           class="flex border-b last:border-b-0 dark:border-slate-800/50 hover:bg-gray-50 dark:hover:bg-slate-800/50 items-start text-xs font-mono transition-colors"
         >
           <span class="px-2 py-0.5 text-slate-500 dark:text-slate-500 shrink-0 border-r dark:border-slate-800/50" :style="{ width: colWidths.ts + 'px' }">{{ log.ts }}</span>
           <span class="px-2 py-0.5 border-r dark:border-slate-800/50 shrink-0" :style="{ width: colWidths.level + 'px' }" :class="levelClass(log.level)">
             {{ log.level }}
           </span>
           <span class="px-2 py-0.5 text-slate-600 dark:text-slate-400 border-r dark:border-slate-800/50 shrink-0 truncate" :style="{ width: colWidths.target + 'px' }" :title="log.target">
             {{ log.target }}
           </span>
           <span class="px-2 py-0.5 text-gray-800 dark:text-gray-300 flex-1 break-words whitespace-pre-wrap">{{ log.message }}</span>
         </div>
       </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, reactive } from 'vue';

interface LogEntry {
  ts: string;
  level: string;
  target: string;
  message: string;
}

const logs = ref<LogEntry[]>([]);
const searchQuery = ref('');
const levelFilter = ref('ALL');
let intervalId: any;

// Column Resizing Logic
const colWidths = reactive({
  ts: 140,
  level: 60,
  target: 120
});

const startResize = (col: keyof typeof colWidths, event: MouseEvent) => {
  const startX = event.clientX;
  const startWidth = colWidths[col];

  const onMouseMove = (moveEvent: MouseEvent) => {
    const diff = moveEvent.clientX - startX;
    colWidths[col] = Math.max(50, startWidth + diff); // Min width 50px
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
    const res = await fetch('/logs');
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
    case "ERROR": return "text-red-600 dark:text-red-400 font-bold";
    case "WARN": return "text-amber-600 dark:text-amber-400 font-bold";
    case "INFO": return "text-green-600 dark:text-green-400 font-bold";
    case "DEBUG": return "text-blue-600 dark:text-blue-400";
    case "TRACE": return "text-purple-600 dark:text-purple-400";
    default: return "text-slate-600 dark:text-slate-400";
  }
};

onMounted(() => {
  fetchLogs();
  intervalId = setInterval(fetchLogs, 1000); // 1s refresh
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
  background-color: rgba(156, 163, 175, 0.5); /* gray-400 */
}
</style>
