<template>
  <div class="h-full overflow-y-auto p-4 lg:p-6 text-slate-900 dark:text-slate-100 bg-slate-50/50 dark:bg-slate-950/50">
    <div class="max-w-6xl mx-auto space-y-6">

      <!-- Control Center (Hero Section) -->
      <section
        class="relative overflow-hidden bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-3xl p-6 lg:p-8 shadow-xl shadow-slate-200/50 dark:shadow-none">
        <!-- Background decoration -->
        <div class="absolute top-0 right-0 -mr-20 -mt-20 w-80 h-80 bg-blue-500/5 rounded-full blur-3xl"></div>
        <div class="absolute bottom-0 left-0 -ml-20 -mb-20 w-80 h-80 bg-amber-500/5 rounded-full blur-3xl"></div>

        <div class="relative grid grid-cols-1 lg:grid-cols-2 gap-8 lg:gap-20 items-center">
          <!-- Left: VPN Connection -->
          <div class="flex items-center gap-6">
            <div :class="[
              'w-16 h-16 lg:w-20 lg:h-20 rounded-2xl flex items-center justify-center transition-all duration-700 shadow-inner shrink-0',
              connectionStatus === 'Connected' ? 'bg-emerald-50 dark:bg-emerald-500/10 text-emerald-500 shadow-emerald-200/50' :
                connectionStatus === 'Connecting' ? 'bg-blue-50 dark:bg-blue-500/10 text-blue-500 transition-all' :
                  'bg-slate-50 dark:bg-slate-800/50 text-slate-300 dark:text-slate-600'
            ]">
              <component :is="connectionStatus === 'Connected' ? ShieldCheckIcon : connectionStatus === 'Connecting' ? ArrowPathIcon : ShieldExclamationIcon" 
                :class="['h-8 w-8 shrink-0', connectionStatus === 'Connecting' ? 'animate-spin' : '']" />
            </div>
            <div class="space-y-1">
              <h3 class="text-[10px] font-bold uppercase tracking-widest text-slate-400">VPN PROTECTION</h3>
              <p :class="[
                'text-xl lg:text-2xl font-black tracking-tight',
                connectionStatus === 'Connected' ? 'text-emerald-600 dark:text-emerald-400' :
                  connectionStatus === 'Connecting' ? 'text-blue-600 dark:text-blue-400' :
                    'text-slate-800 dark:text-slate-100'
              ]">
                {{ connectionStatus === 'Connected' ? 'System Secured' : connectionStatus === 'Connecting' ?
                  'Connecting...' : 'Disconnected' }}
              </p>
              <button @click="toggleConnection" :disabled="connectionStatus === 'Connecting'" :class="[
                'mt-2 px-6 py-2 rounded-xl text-sm font-bold transition-all active:scale-95 flex items-center gap-2 group',
                connectionStatus === 'Connected' ? 'bg-rose-500 text-white hover:bg-rose-600 shadow-lg shadow-rose-500/20' :
                  'bg-blue-600 text-white hover:bg-blue-700 shadow-lg shadow-blue-500/20'
              ]">
                {{ connectionStatus === 'Connected' ? 'Disconnect' : 'Connect' }}
                <component :is="connectionStatus === 'Connected' ? PowerIcon : BoltIcon"
                  class="h-4 w-4 shrink-0" />
              </button>
            </div>
          </div>

          <!-- Right: Gateway Control -->
          <div class="flex items-center gap-6 lg:border-l lg:border-slate-100 lg:dark:border-slate-800 lg:pl-20">
            <div :class="[
              'w-16 h-16 lg:w-20 lg:h-20 rounded-2xl flex items-center justify-center transition-all duration-700 shrink-0',
              gatewayEnabled ? 'bg-amber-50 dark:bg-amber-500/10 text-amber-500 shadow-amber-200/30' : 'bg-slate-50 dark:bg-slate-800/50 text-slate-300 dark:text-slate-600'
            ]">
              <component :is="ServerIcon" class="h-8 w-8 shrink-0" />
            </div>
            <div class="space-y-1">
              <h3 class="text-[10px] font-bold uppercase tracking-widest text-slate-400">GATEWAY SERVICE</h3>
              <p class="text-xl lg:text-2xl font-black tracking-tight">
                {{ gatewayEnabled ? 'Gateway Active' : 'Gateway Off' }}
              </p>
              <button @click="toggleGateway"
                :disabled="gatewayToggling || (connectionStatus !== 'Connected' && !gatewayEnabled)" :class="[
                  'mt-2 px-6 py-2 rounded-xl text-sm font-bold transition-all active:scale-95 disabled:opacity-30 flex items-center gap-2 group',
                  gatewayEnabled ? 'bg-amber-500 text-white hover:bg-amber-600 shadow-lg shadow-amber-500/20' :
                    'bg-slate-100 dark:bg-slate-800 text-slate-600 dark:text-slate-400 hover:bg-slate-200 dark:hover:bg-slate-700'
                ]">
                <div v-if="gatewayToggling" class="flex items-center gap-2">
                  <svg class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
                    <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
                    <path class="opacity-75" fill="currentColor"
                      d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z">
                    </path>
                  </svg>
                </div>
                <span v-else>{{ gatewayEnabled ? 'Disable Gateway' : 'Enable Gateway' }}</span>
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- Quick Stats Grid -->
      <div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-6">
        <StatCard title="Received" :value="formatBytes(rx)" icon="download"
          class="ring-1 ring-slate-100 dark:ring-slate-800/50" />
        <StatCard title="Transmitted" :value="formatBytes(tx)" icon="upload"
          class="ring-1 ring-slate-100 dark:ring-slate-800/50" />
        <StatCard title="Subscription" :value="userExpire" icon="calendar"
          class="ring-1 ring-slate-100 dark:ring-slate-800/50" />
        <StatCard title="Device ID" :value="deviceUuid" icon="cpu"
          class="ring-1 ring-slate-100 dark:ring-slate-800/50" />
      </div>

      <!-- Server Network Section -->
      <section class="space-y-6">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3">
            <div class="p-2.5 bg-blue-500/10 text-blue-500 rounded-xl">
              <component :is="GlobeAltIcon" class="h-5 w-5 shrink-0" />
            </div>
            <div>
              <h2 class="text-xl font-black tracking-tight">Infrastructure</h2>
              <p class="text-sm text-slate-400 font-medium">Global low-latency network nodes</p>
            </div>
          </div>
          <button @click="runSpeedTest" :disabled="speedTesting"
            class="px-5 py-2.5 bg-white dark:bg-slate-900 hover:bg-slate-50 dark:hover:bg-slate-800 border border-slate-200 dark:border-slate-800 text-slate-700 dark:text-slate-300 rounded-xl transition-all flex items-center gap-2 text-sm font-bold shadow-sm active:scale-95 disabled:opacity-50">
            <svg v-if="speedTesting" class="animate-spin h-4 w-4" xmlns="http://www.w3.org/2000/svg" fill="none"
              viewBox="0 0 24 24">
              <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
              <path class="opacity-75" fill="currentColor"
                d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z">
              </path>
            </svg>
            <component v-else :is="CommandLineIcon" class="h-4 w-4 shrink-0" />
            {{ speedTesting ? 'Testing Network...' : 'Speed Test' }}
          </button>
        </div>

        <div
          class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-3xl overflow-hidden shadow-sm">
          <table class="w-full text-left border-collapse">
            <thead>
              <tr class="bg-slate-50/50 dark:bg-slate-800/30">
                <th class="px-8 py-5 text-[11px] font-black text-slate-400 uppercase tracking-[0.2em]">Node Location
                </th>
                <th class="px-8 py-5 text-[11px] font-black text-slate-400 uppercase tracking-[0.2em]">Network Topology
                </th>
                <th class="px-8 py-5 text-[11px] font-black text-slate-400 uppercase tracking-[0.2em]">Performance</th>
                <th class="px-8 py-5 text-[11px] font-black text-slate-400 uppercase tracking-[0.2em] text-right">
                  Selection</th>
              </tr>
            </thead>
            <tbody class="divide-y divide-slate-100 dark:divide-slate-800/50">
              <tr v-for="item in speedResults" :key="item.ip4 + item.ip6"
                class="group hover:bg-slate-50/50 dark:hover:bg-slate-800/20 transition-all duration-300">
                <td class="px-8 py-6">
                  <div class="flex items-center gap-4">
                    <div
                      class="w-10 h-10 rounded-xl bg-slate-100 dark:bg-slate-800 flex items-center justify-center text-lg shadow-inner">
                      {{ getLocationEmoji(item.location) }}
                    </div>
                    <div class="flex flex-col">
                      <span class="text-base font-bold text-slate-800 dark:text-slate-100">{{ item.location }}</span>
                      <span class="text-[11px] font-bold text-slate-400 uppercase tracking-wider">#{{ item.id }}</span>
                    </div>
                  </div>
                </td>
                <td class="px-8 py-6">
                  <div class="space-y-1.5">
                    <div class="flex items-center gap-2">
                      <span
                        class="px-1.5 py-0.5 rounded text-[10px] bg-slate-100 dark:bg-slate-800 text-slate-500 font-bold tracking-tight">V4</span>
                      <span class="text-xs font-medium text-slate-600 dark:text-slate-300 tabular-nums">{{ item.ip4
                      }}</span>
                    </div>
                    <div class="flex items-center gap-2">
                      <span
                        class="px-1.5 py-0.5 rounded text-[10px] bg-slate-100 dark:bg-slate-800 text-slate-500 font-bold tracking-tight">V6</span>
                      <span
                        class="text-xs font-medium text-slate-400 dark:text-slate-500 tabular-nums truncate max-w-[120px]"
                        :title="item.ip6">{{ item.ip6 }}</span>
                    </div>
                  </div>
                </td>
                <td class="px-8 py-6">
                  <div class="flex items-center gap-6 font-bold">
                    <div class="flex flex-col gap-1">
                      <span class="text-[10px] text-slate-400 uppercase tracking-widest">Latency IPv4</span>
                      <div class="flex items-center gap-2">
                        <div v-if="item.latency4_ms !== null" class="flex items-center gap-1.5">
                          <div :class="['w-1.5 h-1.5 rounded-full', getStatusColor(item.latency4_ms)]"></div>
                          <span :class="['text-sm tabular-nums', getStatusTextColor(item.latency4_ms)]">{{
                            item.latency4_ms }} ms</span>
                        </div>
                        <span v-else class="text-xs text-slate-300 italic font-medium">Offline</span>
                      </div>
                    </div>
                    <div class="w-px h-8 bg-slate-100 dark:bg-slate-800"></div>
                    <div class="flex flex-col gap-1">
                      <span class="text-[10px] text-slate-400 uppercase tracking-widest">Latency IPv6</span>
                      <div class="flex items-center gap-2">
                        <div v-if="item.latency6_ms !== null" class="flex items-center gap-1.5">
                          <div :class="['w-1.5 h-1.5 rounded-full', getStatusColor(item.latency6_ms)]"></div>
                          <span :class="['text-sm tabular-nums', getStatusTextColor(item.latency6_ms)]">{{
                            item.latency6_ms }} ms</span>
                        </div>
                        <span v-else class="text-xs text-slate-300 italic font-medium">Offline</span>
                      </div>
                    </div>
                  </div>
                </td>
                <td class="px-8 py-6 text-right">
                  <button @click="toggleNode(item.id)" :class="[
                    'relative w-12 h-6 rounded-full transition-all duration-300 ring-2 ring-transparent',
                    enabledNode === item.id ? 'bg-blue-600 ring-blue-500/30' : 'bg-slate-200 dark:bg-slate-800'
                  ]">
                    <div :class="[
                      'absolute top-1 left-1 w-4 h-4 bg-white rounded-full transition-all duration-300 shadow-sm',
                      enabledNode === item.id ? 'translate-x-6 scale-110' : 'translate-x-0'
                    ]"></div>
                  </button>
                </td>
              </tr>
            </tbody>
          </table>

          <div v-if="speedResults.length === 0 && !speedTesting"
            class="px-8 py-20 text-center flex flex-col items-center justify-center gap-4">
            <div class="p-4 bg-slate-50 dark:bg-slate-800/50 rounded-full text-slate-300">
              <svg xmlns="http://www.w3.org/2000/svg" width="48" height="48" viewBox="0 0 24 24" fill="none"
                stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                <circle cx="12" cy="12" r="10" />
                <path d="M12 16v-4" />
                <path d="M12 8h.01" />
              </svg>
            </div>
            <div class="space-y-1">
              <p class="text-lg font-bold text-slate-600 dark:text-slate-300">Network Ready</p>
              <p class="text-sm text-slate-400">Run a speed test to optimize your connection path</p>
            </div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import StatCard from "../components/StatCard.vue";
import {
  GlobeAltIcon,
  ShieldCheckIcon,
  ShieldExclamationIcon,
  ArrowPathIcon,
  ServerIcon,
  BoltIcon,
  PowerIcon,
  CommandLineIcon
} from "@heroicons/vue/24/outline";

const rx = ref(0);
const tx = ref(0);
const speedResults = ref<any[]>([]);
const speedTesting = ref(false);
const enabledNode = ref<string | null>(null);
const userExpire = ref("--");
const deviceUuid = ref("--");
const connectionStatus = ref("Disconnected");
const gatewayEnabled = ref(false);
const gatewayToggling = ref(false);

const getLocationEmoji = (location: string) => {
  const loc = location.toLowerCase();
  if (loc.includes('hong kong') || loc.includes('hk')) return '🇭🇰';
  if (loc.includes('tokyo') || loc.includes('japan') || loc.includes('jp')) return '🇯🇵';
  if (loc.includes('singapore') || loc.includes('sg')) return '🇸🇬';
  if (loc.includes('united states') || loc.includes('us')) return '🇺🇸';
  if (loc.includes('korea') || loc.includes('kr')) return '🇰🇷';
  if (loc.includes('taiwan') || loc.includes('tw')) return '🇹🇼';
  return '🌐';
};

const getStatusColor = (latency: number) => {
  if (latency < 100) return 'bg-emerald-500';
  if (latency < 200) return 'bg-blue-500';
  if (latency < 300) return 'bg-amber-500';
  return 'bg-rose-500';
};

const fetchServers = async () => {
  try {
    const res = await fetch('/api/servers');
    if (res.ok) {
      const servers = await res.json();
      // Initialize speedResults with empty latencies if not already testing/tested
      if (speedResults.value.length === 0) {
        speedResults.value = servers.map((s: any) => ({
          ...s,
          latency4_ms: null,
          latency6_ms: null
        }));
      }
    }
  } catch (e) {
    console.error("Failed to fetch servers:", e);
  }
};

const toggleConnection = async () => {
  const isConnecting = connectionStatus.value === 'Disconnected';
  const url = isConnecting ? '/api/connect' : '/api/disconnect';

  if (isConnecting) connectionStatus.value = 'Connecting';

  try {
    const res = await fetch(url, { method: 'POST' });
    if (res.ok) {
      const data = await res.json();
      if (data.status === 'success' || data.status === 'already_connected') {
        // Stats loop will pick up the new status
      } else if (data.status === 'error' && isConnecting) {
        connectionStatus.value = 'Disconnected';
        alert(data.message || 'Failed to connect');
      }
    }
  } catch (e) {
    console.error("Failed to toggle connection:", e);
    if (isConnecting) connectionStatus.value = 'Disconnected';
  }
};

const toggleGateway = async () => {
  if (connectionStatus.value !== 'Connected' && !gatewayEnabled.value) {
    alert("VPN must be connected to enable gateway mode");
    return;
  }

  const url = gatewayEnabled.value ? '/api/gateway/off' : '/api/gateway/on';
  gatewayToggling.value = true;

  try {
    const res = await fetch(url, { method: 'POST' });
    if (res.ok) {
      // Status will be updated by fetchStats polling
    } else {
      const msg = await res.text();
      alert(msg || 'Failed to toggle gateway');
    }
  } catch (e) {
    console.error("Failed to toggle gateway:", e);
    alert('Failed to toggle gateway');
  } finally {
    gatewayToggling.value = false;
  }
};

const fetchUserInfo = async () => {
  try {
    const res = await fetch('/api/user_info');
    if (res.ok) {
      const data = await res.json();
      userExpire.value = data.expire;
      deviceUuid.value = data.device_id;
    }
  } catch (e) {
    console.error("Failed to fetch user info:", e);
  }
};

const toggleNode = async (id: string) => {
  if (enabledNode.value === id) {
    enabledNode.value = null;
  } else {
    enabledNode.value = id;
    const node = speedResults.value.find(n => n.id === id);
    if (node) {
      const lat4 = node.latency4_ms ?? Infinity;
      const lat6 = node.latency6_ms ?? Infinity;
      // Pick the IP with lower latency. If both are infinity, default to ip4.
      const targetIp = lat4 <= lat6 ? node.ip4 : node.ip6;

      try {
        await fetch('/api/switch_node', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ ip: targetIp })
        });
      } catch (e) {
        console.error("Failed to switch node:", e);
      }
    }
  }
};

const getStatusTextColor = (latency: number) => {
  if (latency < 100) return 'text-emerald-600 dark:text-emerald-400';
  if (latency < 200) return 'text-blue-600 dark:text-blue-400';
  if (latency < 300) return 'text-amber-600 dark:text-amber-400';
  return 'text-rose-600 dark:text-rose-400';
};

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
    const res = await fetch('/api/getwgstats');
    if (res.ok) {
      const data = await res.json();
      rx.value = data.rx;
      tx.value = data.tx;
      connectionStatus.value = data.status;
      gatewayEnabled.value = data.gateway_enabled;
    }
  } catch (e) {
    console.error("Failed to fetch stats:", e);
  }
};

const runSpeedTest = async () => {
  speedTesting.value = true;
  try {
    const res = await fetch('/api/speedtest');
    if (res.ok) {
      speedResults.value = await res.json();
    }
  } catch (e) {
    console.error("Failed to run speed test:", e);
  } finally {
    speedTesting.value = false;
  }
};

onMounted(() => {
  fetchStats();
  fetchUserInfo();
  fetchServers();
  runSpeedTest();
  intervalId = setInterval(fetchStats, 1000);
});

onUnmounted(() => {
  if (intervalId) clearInterval(intervalId);
});
</script>
