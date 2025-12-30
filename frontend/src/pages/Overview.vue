<template>
  <div class="h-full overflow-y-auto p-6 text-slate-900 dark:text-slate-100">
    <div class="grid grid-cols-12 gap-4">
      <StatCard title="Peers" value="1" class="col-span-3" />
      <StatCard title="Revenue" value="$9,876" class="col-span-3" />
      <StatCard title="RX" :value="formatBytes(rx)" class="col-span-3" />
      <StatCard title="TX" :value="formatBytes(tx)" class="col-span-3" />

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
import { ref, onMounted, onUnmounted } from 'vue';
import StatCard from "../components/StatCard.vue";

const rx = ref(0);
const tx = ref(0);

const formatBytes = (bytes: number, decimals = 2) => {
    if (!+bytes) return '0 B';
    const k = 1024;
    const dm = decimals < 0 ? 0 : decimals;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB', 'PB', 'EB', 'ZB', 'YB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

let intervalId: any;

const fetchStats = async () => {
  try {
    const res = await fetch('/getwgstats');
    if (res.ok) {
      const data = await res.json();
      rx.value = data.rx;
      tx.value = data.tx;
    }
  } catch (e) {
    console.error("Failed to fetch stats:", e);
  }
};

onMounted(() => {
  fetchStats();
  intervalId = setInterval(fetchStats, 1000);
});

onUnmounted(() => {
  if (intervalId) clearInterval(intervalId);
});
</script>
