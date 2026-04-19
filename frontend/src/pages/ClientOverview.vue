<!--
  Client Mode - Overview Dashboard
  Main landing page for client mode users
  Shows: Connection control, quick endpoint selector, enhance mode status
-->

<template>
  <div class="h-full overflow-y-auto p-4 lg:p-6 text-slate-900 dark:text-slate-100 bg-slate-50/50 dark:bg-slate-950/50">
    <div class="max-w-6xl mx-auto space-y-6"> <!-- Status Cards Row (Reduced) -->
      <section class="grid grid-cols-1 md:grid-cols-3 gap-4">

        <!-- Upload Traffic Card -->
        <div
          class="relative overflow-hidden bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-lg shadow-slate-200/50 dark:shadow-none">
          <div class="flex items-center gap-3">
            <div
              class="w-10 h-10 rounded-xl bg-sky-50 dark:bg-sky-500/10 text-sky-500 flex items-center justify-center shrink-0">
              <ArrowUpIcon class="h-5 w-5 shrink-0" stroke-width="2.5" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-widest">Upload</p>
              <p class="text-sm font-bold text-slate-800 dark:text-slate-100">{{ uploadDisplay }}</p>
              <p class="text-xs text-slate-500 dark:text-slate-400">Current upstream traffic</p>
            </div>
          </div>
        </div>

        <!-- Download Traffic Card -->
        <div
          class="relative overflow-hidden bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-lg shadow-slate-200/50 dark:shadow-none">
          <div class="flex items-center gap-3">
            <div
              class="w-10 h-10 rounded-xl bg-indigo-50 dark:bg-indigo-500/10 text-indigo-500 flex items-center justify-center shrink-0">
              <ArrowDownIcon class="h-5 w-5 shrink-0" stroke-width="2.5" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-widest">Download</p>
              <p class="text-sm font-bold text-slate-800 dark:text-slate-100">{{ downloadDisplay }}</p>
              <p class="text-xs text-slate-500 dark:text-slate-400">Current downstream traffic</p>
            </div>
          </div>
        </div>

        <!-- Current Node Card -->
        <div
          class="relative overflow-hidden bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-lg shadow-slate-200/50 dark:shadow-none">
          <div class="flex items-center gap-3">
            <div
              class="w-10 h-10 rounded-xl bg-blue-50 dark:bg-blue-500/10 text-blue-500 flex items-center justify-center shrink-0">
              <MapPinIcon class="h-5 w-5 shrink-0" stroke-width="2" />
            </div>
            <div class="flex-1 min-w-0">
              <p class="text-xs font-semibold text-slate-500 dark:text-slate-400 uppercase tracking-widest">Current Node
              </p>
              <p class="text-sm font-bold text-slate-800 dark:text-slate-100 truncate">{{ selectedEndpointDisplay }}</p>
              <p class="text-xs text-slate-500 dark:text-slate-400 truncate">{{ selectedEndpointInfo }}</p>
            </div>
          </div>
        </div>

      </section>
      <!-- Control Switches Row -->
      <section class="grid grid-cols-1 md:grid-cols-3 gap-4">

        <!-- Protocol Switch -->
        <div
          class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-lg shadow-slate-200/50 dark:shadow-none">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div
                class="w-10 h-10 rounded-xl bg-cyan-50 dark:bg-cyan-500/10 text-cyan-500 flex items-center justify-center shrink-0">
                <AdjustmentsHorizontalIcon class="h-5 w-5 shrink-0" stroke-width="2" />
              </div>
              <div>
                <p class="text-sm font-semibold text-slate-800 dark:text-slate-100">Protocol</p>
                <p class="text-xs text-slate-500 dark:text-slate-400">TCP / UDP</p>
              </div>
            </div>
            <button @click="toggleProtocol" :disabled="wireguardStore.isConnected" :class="[
              'relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-cyan-500 focus:ring-offset-2',
              settingsStore.protocol === 'tcp' ? 'bg-cyan-600' : 'bg-slate-200 dark:bg-slate-700',
              wireguardStore.isConnected ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'
            ]">
              <span :class="[
                'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
                settingsStore.protocol === 'tcp' ? 'translate-x-6' : 'translate-x-1'
              ]" />
            </button>
          </div>
          <div class="mt-3 flex items-center justify-between text-xs">
            <span
              :class="settingsStore.protocol === 'udp' ? 'text-cyan-600 dark:text-cyan-400 font-semibold' : 'text-slate-500'">UDP</span>
            <span
              :class="settingsStore.protocol === 'tcp' ? 'text-cyan-600 dark:text-cyan-400 font-semibold' : 'text-slate-500'">TCP</span>
          </div>
        </div>

        <!-- Proxy Mode Switch -->
        <div
          class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-lg shadow-slate-200/50 dark:shadow-none">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div
                class="w-10 h-10 rounded-xl bg-purple-50 dark:bg-purple-500/10 text-purple-500 flex items-center justify-center shrink-0">
                <ArrowsRightLeftIcon class="h-5 w-5 shrink-0" stroke-width="2" />
              </div>
              <div>
                <p class="text-sm font-semibold text-slate-800 dark:text-slate-100">Proxy Mode</p>
                <p class="text-xs text-slate-500 dark:text-slate-400">Global / Split</p>
              </div>
            </div>
            <button @click="toggleProxyMode" :disabled="wireguardStore.isConnected" :class="[
              'relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-purple-500 focus:ring-offset-2',
              settingsStore.proxyMode === 'global' ? 'bg-purple-600' : 'bg-slate-200 dark:bg-slate-700',
              wireguardStore.isConnected ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'
            ]">
              <span :class="[
                'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
                settingsStore.proxyMode === 'global' ? 'translate-x-6' : 'translate-x-1'
              ]" />
            </button>
          </div>
          <div class="mt-3 flex items-center justify-between text-xs">
            <span
              :class="settingsStore.proxyMode === 'split' ? 'text-purple-600 dark:text-purple-400 font-semibold' : 'text-slate-500'">Split</span>
            <span
              :class="settingsStore.proxyMode === 'global' ? 'text-purple-600 dark:text-purple-400 font-semibold' : 'text-slate-500'">Global</span>
          </div>
        </div>

        <!-- Obfuscation Switch -->
        <div
          class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-lg shadow-slate-200/50 dark:shadow-none">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div :class="[
                'w-10 h-10 rounded-xl flex items-center justify-center shrink-0',
                settingsStore.obfuscate ? 'bg-emerald-50 dark:bg-emerald-500/10 text-emerald-500' : 'bg-slate-50 dark:bg-slate-800/50 text-slate-400'
              ]">
                <component :is="settingsStore.obfuscate ? ShieldCheckIcon : ShieldExclamationIcon" class="h-5 w-5 shrink-0" stroke-width="2" />
              </div>
              <div>
                <p class="text-sm font-semibold text-slate-800 dark:text-slate-100">Obfuscation</p>
                <p class="text-xs text-slate-500 dark:text-slate-400">Enabled / Disabled</p>
              </div>
            </div>
            <button @click="toggleObfuscation" :disabled="wireguardStore.isConnected" :class="[
              'relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-emerald-500 focus:ring-offset-2',
              settingsStore.obfuscate ? 'bg-emerald-600' : 'bg-slate-200 dark:bg-slate-700',
              wireguardStore.isConnected ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'
            ]">
              <span :class="[
                'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
                settingsStore.obfuscate ? 'translate-x-6' : 'translate-x-1'
              ]" />
            </button>
          </div>
          <div class="mt-3 flex items-center justify-between text-xs">
            <span
              :class="!settingsStore.obfuscate ? 'text-emerald-600 dark:text-emerald-400 font-semibold' : 'text-slate-500'">Off</span>
            <span
              :class="settingsStore.obfuscate ? 'text-emerald-600 dark:text-emerald-400 font-semibold' : 'text-slate-500'">On</span>
          </div>
        </div>

        <!-- LAN Gateway -->
        <div
          class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl p-4 shadow-lg shadow-slate-200/50 dark:shadow-none">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-3">
              <div :class="[
                'w-10 h-10 rounded-xl flex items-center justify-center shrink-0 transition-colors',
                wireguardStore.gatewayEnabled ? 'bg-amber-50 dark:bg-amber-500/10 text-amber-500' : 'bg-slate-50 dark:bg-slate-800/50 text-slate-400'
              ]">
                <ServerIcon class="h-5 w-5 shrink-0" stroke-width="2" />
              </div>
              <div>
                <p class="text-sm font-semibold text-slate-800 dark:text-slate-100">LAN Gateway</p>
                <p class="text-xs text-slate-500 dark:text-slate-400">Enabled / Disabled</p>
              </div>
            </div>
            <button @click="wireguardStore.toggleGateway" :disabled="!wireguardStore.isConnected && !wireguardStore.gatewayEnabled" :class="[
              'relative inline-flex h-6 w-11 items-center rounded-full transition-colors focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2',
              wireguardStore.gatewayEnabled ? 'bg-amber-600' : 'bg-slate-200 dark:bg-slate-700',
              (!wireguardStore.isConnected && !wireguardStore.gatewayEnabled) ? 'opacity-50 cursor-not-allowed' : 'cursor-pointer'
            ]">
              <span :class="[
                'inline-block h-4 w-4 transform rounded-full bg-white transition-transform',
                wireguardStore.gatewayEnabled ? 'translate-x-6' : 'translate-x-1'
              ]" />
            </button>
          </div>
          <div class="mt-3 flex items-center justify-between text-xs">
            <span
              :class="!wireguardStore.gatewayEnabled ? 'text-amber-600 dark:text-amber-400 font-semibold' : 'text-slate-500'">Off</span>
            <span
              :class="wireguardStore.gatewayEnabled ? 'text-amber-600 dark:text-amber-400 font-semibold' : 'text-slate-500'">On</span>
          </div>
        </div>
      </section>

      <!-- Connection Control Section -->
      <section
        class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-3xl p-6 lg:p-8 shadow-xl shadow-slate-200/50 dark:shadow-none">
        <div class="text-center space-y-4">
          <button @click="toggleConnection" :disabled="wireguardStore.isConnecting" :class="[
            'px-8 py-4 rounded-2xl text-lg font-bold transition-all active:scale-95 flex items-center gap-3 mx-auto group',
            wireguardStore.isConnected ? 'bg-rose-500 text-white hover:bg-rose-600 shadow-lg shadow-rose-500/20' :
              'bg-blue-600 text-white hover:bg-blue-700 shadow-lg shadow-blue-500/20'
          ]">
            <component :is="wireguardStore.isConnected ? PowerIcon : BoltIcon" class="h-6 w-6 shrink-0" stroke-width="2.5" />
            {{ wireguardStore.isConnected ? 'Disconnect' : 'Connect' }}
          </button>

          <!-- Error Message -->
          <div v-if="wireguardStore.error"
            class="mt-4 p-4 rounded-lg bg-red-100 dark:bg-red-500/10 border border-red-300 dark:border-red-500/30 text-red-700 dark:text-red-400">
            <p class="text-sm font-medium">{{ wireguardStore.error }}</p>
          </div>
        </div>
      </section>

    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, onUnmounted } from 'vue';
import { useWireGuardMode } from '@/composables/useWireGuardMode';
import { 
  ArrowUpIcon, 
  ArrowDownIcon,
  MapPinIcon,
  AdjustmentsHorizontalIcon,
  ArrowsRightLeftIcon,
  ShieldCheckIcon,
  ShieldExclamationIcon,
  ServerIcon,
  BoltIcon,
  PowerIcon
} from "@heroicons/vue/24/outline";

const { wireguardStore, endpointsStore, settingsStore } = useWireGuardMode();

const rxBytes = ref(0);
const txBytes = ref(0);

const selectedEndpointDisplay = computed(() => {
  if (endpointsStore.selectedEndpoint) {
    return endpointsStore.selectedEndpoint.name;
  }
  return 'No endpoint selected';
});

const selectedEndpointInfo = computed(() => {
  if (endpointsStore.selectedEndpoint) {
    const ep = endpointsStore.selectedEndpoint;
    return `${ep.address}:${ep.port}${ep.location ? ` • ${ep.location}` : ''}`;
  }
  return 'Select an endpoint to connect';
});

const uploadDisplay = computed(() => formatBytes(txBytes.value));
const downloadDisplay = computed(() => formatBytes(rxBytes.value));

let statsInterval: any;

async function fetchStats() {
  try {
    const stats = await wireguardStore.fetchStats();
    if (stats) {
      rxBytes.value = stats.rx;
      txBytes.value = stats.tx;
    }
  } catch (err) {
    console.error('Failed to fetch stats:', err);
  }
}

onMounted(() => {
  fetchStats();
  statsInterval = setInterval(fetchStats, 2000);
});

onUnmounted(() => {
  if (statsInterval) clearInterval(statsInterval);
});

function formatBytes(bytes: number, decimals = 2) {
  if (!bytes) return '0 B';
  const k = 1024;
  const dm = decimals < 0 ? 0 : decimals;
  const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
  const i = Math.floor(Math.log(bytes) / Math.log(k));
  return `${parseFloat((bytes / Math.pow(k, i)).toFixed(dm))} ${sizes[i]}`;
}

async function toggleConnection() {
  try {
    if (wireguardStore.isConnected) {
      await wireguardStore.disconnect();
    } else {
      const endpoint = endpointsStore.selectedEndpoint;
      if (endpoint && endpoint.id) {
        await wireguardStore.connect(endpoint.id);
      } else {
        wireguardStore.error = endpoint ? 'Invalid endpoint configuration' : 'Please select an endpoint first';
      }
    }
  } catch (err) {
    console.error('Connection toggle failed:', err);
  }
}

async function toggleProtocol() {
  try {
    const newProtocol = settingsStore.protocol === 'tcp' ? 'udp' : 'tcp';
    settingsStore.setProtocol(newProtocol);
    await settingsStore.saveSettings();
  } catch (err) {
    console.error('Protocol toggle failed:', err);
  }
}

async function toggleProxyMode() {
  try {
    const newMode = settingsStore.proxyMode === 'global' ? 'split' : 'global';
    settingsStore.setProxyMode(newMode);
    await settingsStore.saveSettings();
  } catch (err) {
    console.error('Proxy mode toggle failed:', err);
  }
}

async function toggleObfuscation() {
  try {
    settingsStore.toggleObfuscate();
    await settingsStore.saveSettings();
  } catch (err) {
    console.error('Obfuscation toggle failed:', err);
  }
}
</script>
