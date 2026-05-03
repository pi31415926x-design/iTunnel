<template>
  <div class="h-full overflow-y-auto p-4 lg:p-8 text-slate-900 bg-slate-50/50 dark:bg-slate-950/50 dark:text-slate-100">
    <div class="max-w-5xl mx-auto space-y-6">

      <!-- Stats Row -->
      <section class="grid grid-cols-1 md:grid-cols-3 gap-4">

        <!-- Active Peers Card -->
        <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-sm p-5">
          <div class="flex items-center gap-4">
            <div class="w-11 h-11 rounded-lg dark:bg-red-500/10 text-red-600 dark:text-red-400 flex items-center justify-center shrink-0">
              <UsersIcon class="h-5 w-5 shrink-0" stroke-width="2" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">Active Peers</p>
              <p class="mt-0.5 text-lg font-bold text-slate-800 dark:text-slate-100 truncate">{{ peersDisplay }}</p>
              <p class="text-xs text-slate-500 dark:text-slate-400">Clients currently routing</p>
            </div>
          </div>
        </div>

        <!-- RX Traffic Card -->
        <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-sm p-5">
          <div class="flex items-center gap-4">
            <div class="w-11 h-11 rounded-lg dark:bg-red-500/10 text-red-600 dark:text-red-400 flex items-center justify-center shrink-0">
              <ArrowDownIcon class="h-5 w-5 shrink-0" stroke-width="2" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">Total RX</p>
              <p class="mt-0.5 text-lg font-bold text-slate-800 dark:text-slate-100 truncate">{{ downloadDisplay }}</p>
              <p class="text-xs text-slate-500 dark:text-slate-400">Total received bytes</p>
            </div>
          </div>
        </div>

        <!-- TX Traffic Card -->
        <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-sm p-5">
          <div class="flex items-center gap-4">
            <div class="w-11 h-11 rounded-lg dark:bg-red-500/10 text-red-600 dark:text-red-400 flex items-center justify-center shrink-0">
              <ArrowUpIcon class="h-5 w-5 shrink-0" stroke-width="2" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">Total TX</p>
              <p class="mt-0.5 text-lg font-bold text-slate-800 dark:text-slate-100 truncate">{{ uploadDisplay }}</p>
              <p class="text-xs text-slate-500 dark:text-slate-400">Total transmitted bytes</p>
            </div>
          </div>
        </div>
      </section>

      <!-- Top Row 2: Protocol Obfuscation -->
      <section class="grid grid-cols-1 md:grid-cols-2 gap-4">
        <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-sm p-5">
          <div class="flex items-center gap-4">
            <div class="w-11 h-11 rounded-lg dark:bg-red-500/10 text-red-600 dark:text-red-400 flex items-center justify-center shrink-0">
              <CpuChipIcon class="h-5 w-5 shrink-0" stroke-width="2" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">Obfuscation</p>
              <p class="mt-0.5 text-lg font-bold text-slate-800 dark:text-slate-100 truncate">{{ protocolObfuscation ? 'Enabled' : 'Disabled' }}</p>
              <p class="text-xs text-slate-500 dark:text-slate-400">Applies when starting server</p>
            </div>
            <div class="shrink-0">
              <button
                @click="protocolObfuscation = !protocolObfuscation"
                :disabled="isToggling || serverRunning"
                :class="[
                  protocolObfuscation ? 'bg-red-700' : 'bg-slate-200 dark:bg-slate-700',
                  (isToggling || serverRunning) ? 'opacity-50 cursor-not-allowed' : '',
                  'relative inline-flex h-7 w-12 items-center rounded-full transition-colors'
                ]"
                title="Toggle Protocol Obfuscation"
              >
                <span :class="[
                  protocolObfuscation ? 'translate-x-6 bg-white' : 'translate-x-1 bg-white dark:bg-slate-400',
                  'inline-block h-5 w-5 transform rounded-full transition-transform shadow-sm'
                ]" />
              </button>
            </div>
          </div>
        </div>
        <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl shadow-sm p-5">
          <div class="flex items-center gap-4">
            <div class="w-11 h-11 rounded-lg dark:bg-red-500/10 text-red-600 dark:text-red-400 flex items-center justify-center shrink-0">
              <ArrowPathIcon class="h-5 w-5 shrink-0" stroke-width="2" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-wider">TCP Mode</p>
              <p class="mt-0.5 text-lg font-bold text-slate-800 dark:text-slate-100 truncate">{{ tcpMode ? 'Enabled' : 'Disabled' }}</p>
              <p class="text-xs text-slate-500 dark:text-slate-400">Use TCP as transport</p>
            </div>
            <div class="shrink-0">
              <button
                @click="tcpMode = !tcpMode"
                :disabled="isToggling || serverRunning"
                :class="[
                  tcpMode ? 'bg-red-700' : 'bg-slate-200 dark:bg-slate-700',
                  (isToggling || serverRunning) ? 'opacity-50 cursor-not-allowed' : '',
                  'relative inline-flex h-7 w-12 items-center rounded-full transition-colors'
                ]"
                title="Toggle TCP Mode"
              >
                <span :class="[
                  tcpMode ? 'translate-x-6 bg-white' : 'translate-x-1 bg-white dark:bg-slate-400',
                  'inline-block h-5 w-5 transform rounded-full transition-transform shadow-sm'
                ]" />
              </button>
            </div>
          </div>
        </div>
      </section>

      <!-- Service Control Card -->
      <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl overflow-hidden shadow-sm">

        <!-- Header -->
        <div class="px-6 py-5 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between">
          <div class="flex items-center gap-3">
            <h2 class="text-2xl font-bold">Service Control</h2>
            <span :class="[
              'inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium',
              serverRunning
                ? 'bg-red-50 text-red-700 dark:bg-red-500/10 dark:text-red-400'
                : 'bg-slate-100 text-slate-600 dark:bg-slate-800 dark:text-slate-400'
            ]">
              <span :class="[
                'w-1.5 h-1.5 rounded-full',
                serverRunning ? 'bg-red-600' : 'bg-slate-400'
              ]"></span>
              {{ serverRunning ? 'Running' : 'Stopped' }}
            </span>
          </div>
        </div>

        <!-- Body -->
        <div class="px-6 py-6">
          <p class="text-sm text-slate-500 dark:text-slate-400 mb-5">
            Toggle the WireGuard server instance. When running, this node accepts connections from configured peers
            according to your server identity and peer list.
          </p>

          <div class="flex justify-center">
            <button
              @click="toggleServer"
              :disabled="isToggling"
              class="inline-flex items-center gap-2 px-6 py-3 text-medium font-medium text-white bg-red-600 hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg shadow-sm shadow-red-500/20 transition-all hover:shadow-red-500/40"
            >
              <ArrowPathIcon v-if="isToggling" class="w-4 h-4 animate-spin" />
              <component v-else :is="serverRunning ? PowerIcon : PlayIcon" class="w-4 h-4" />
              <span v-if="isToggling">Processing...</span>
              <span v-else>{{ serverRunning ? 'Stop Server' : 'Start Server' }}</span>
            </button>
          </div>
        </div>
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { serverFetch } from '@/server-fetch';
import {
  ArrowUpIcon,
  ArrowDownIcon,
  UsersIcon,
  PlayIcon,
  PowerIcon,
  ArrowPathIcon,
  CpuChipIcon
} from '@heroicons/vue/20/solid';

const serverRunning = ref(false);
const rxBytes = ref(0);
const txBytes = ref(0);
const peersCount = ref(0);
const isToggling = ref(false);
const protocolObfuscation = ref(false);
const tcpMode = ref(false);

const uploadDisplay = computed(() => formatBytes(txBytes.value));
const downloadDisplay = computed(() => formatBytes(rxBytes.value));
const peersDisplay = computed(() => serverRunning.value ? peersCount.value.toString() : 'Offline');

function formatBytes(bytes: number, decimals = 2) {
  if (!bytes) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

let statusInterval: ReturnType<typeof setInterval> | null = null;

async function fetchStatus() {
  try {
    const res = await serverFetch('/api/server_status');
    const json = await res.json();
    if (json.success) {
      serverRunning.value = json.running;
      rxBytes.value = json.rx_bytes;
      txBytes.value = json.tx_bytes;
      peersCount.value = json.peers_count;
    }
  } catch (err) {
    console.error('Failed to fetch server status:', err);
  }
}

onMounted(() => {
  fetchStatus();
  statusInterval = setInterval(fetchStatus, 3000);
});

onUnmounted(() => {
  if (statusInterval) clearInterval(statusInterval);
});

async function toggleServer() {
  if (isToggling.value) return;
  isToggling.value = true;

  const endpoint = serverRunning.value ? '/api/stop' : '/api/start';

  try {
    const res = await serverFetch(endpoint, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: serverRunning.value
        ? undefined
        : JSON.stringify({
            protocol_obfuscation: protocolObfuscation.value,
            use_tcp: tcpMode.value,
          }),
    });
    const json = await res.json();

    if (json.success) {
      await fetchStatus();
    } else {
      alert(`Operation Failed: ${json.message || 'Unknown error'}`);
    }
  } catch (err) {
    console.error('Toggle attempt failed:', err);
    alert('Failed to connect to the local backend.');
  } finally {
    isToggling.value = false;
  }
}
</script>
