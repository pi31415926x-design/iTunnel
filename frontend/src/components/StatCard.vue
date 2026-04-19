<template>
  <div
    class="relative group overflow-hidden bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-6 transition-all duration-300 hover:border-blue-500/30 hover:shadow-lg hover:shadow-blue-500/5">
    <div class="flex items-start justify-between">
      <div class="space-y-2">
        <p class="text-xs font-bold text-slate-400 uppercase tracking-widest">{{ title }}</p>
        <p class="text-xs font-black text-slate-800 dark:text-slate-100 tracking-tight">{{ value }}</p>
      </div>
      <div v-if="icon"
        class="p-2.5 rounded-xl bg-slate-50 dark:bg-slate-800/50 text-slate-400 dark:text-slate-500 group-hover:bg-blue-50 group-hover:text-blue-500 dark:group-hover:bg-blue-500/10 transition-colors">
        <component :is="iconComponent" class="h-5 w-5 shrink-0" stroke-width="2.5" />
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { 
  ArrowUpIcon, 
  ArrowDownIcon, 
  CalendarIcon, 
  CpuChipIcon 
} from "@heroicons/vue/24/outline";

const props = defineProps<{
  title: string;
  value: string;
  icon?: 'upload' | 'download' | 'calendar' | 'cpu';
}>();

const iconComponent = computed(() => {
  switch (props.icon) {
    case 'upload': return ArrowUpIcon;
    case 'download': return ArrowDownIcon;
    case 'calendar': return CalendarIcon;
    case 'cpu': return CpuChipIcon;
    default: return null;
  }
});
</script>
