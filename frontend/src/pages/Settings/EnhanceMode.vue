<!--
  Client Mode - Enhance Mode Settings
  Configure WireGuard enhancements: TCP/UDP, Obfuscation, Proxy Mode
-->

<template>
  <div class="h-full overflow-y-auto p-4 lg:p-6 text-slate-900 dark:text-slate-100 bg-slate-50/50 dark:bg-slate-950/50">
    <div class="max-w-2xl mx-auto space-y-6">

      <!-- Header -->
      <div class="flex items-start justify-between">
        <div>
          <h1 class="text-3xl font-bold">Enhance Mode Settings</h1>
          <p class="text-slate-600 dark:text-slate-400 mt-1">Optimize your WireGuard connection</p>
        </div>
        <button 
          v-if="settingsStore.hasChanges"
          @click="saveSettings"
          :disabled="settingsStore.saving"
          class="px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-white font-medium transition-all disabled:opacity-50 flex items-center gap-2">
          <svg v-if="settingsStore.saving" class="animate-spin w-4 h-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z"></path>
          </svg>
          <span>{{ settingsStore.saving ? 'Saving...' : 'Save Changes' }}</span>
        </button>
      </div>

      <!-- Error Message -->
      <div v-if="settingsStore.error" class="p-4 rounded-lg bg-red-100 dark:bg-red-500/10 border border-red-300 dark:border-red-500/30 text-red-700 dark:text-red-400">
        <p class="text-sm font-medium">{{ settingsStore.error }}</p>
        <button @click="settingsStore.clearError()" class="text-xs mt-2 underline">Dismiss</button>
      </div>

      <!-- Settings Card -->
      <div class="bg-white dark:bg-slate-900 rounded-lg border border-slate-200 dark:border-slate-800 p-6 space-y-6">

        <!-- 1. Protocol Selection -->
        <div>
          <div class="flex items-center justify-between mb-4">
            <div>
              <h2 class="text-lg font-semibold">Protocol</h2>
              <p class="text-sm text-slate-600 dark:text-slate-400 mt-1">
                Choose between UDP (faster) and TCP (more compatible)
              </p>
            </div>
            <div class="flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-blue-500">
                <path d="M12 3v18M3 12h18"></path>
              </svg>
            </div>
          </div>

          <div class="grid grid-cols-2 gap-3">
            <button
              @click="settingsStore.setProtocol('udp')"
              :class="[
                'px-4 py-3 rounded-lg border-2 font-semibold transition-all',
                settingsStore.protocol === 'udp'
                  ? 'border-blue-500 bg-blue-50 dark:bg-blue-500/10 text-blue-700 dark:text-blue-400'
                  : 'border-slate-300 dark:border-slate-700 text-slate-700 dark:text-slate-300 hover:border-slate-400'
              ]">
              UDP (Default)
            </button>
            <button
              @click="settingsStore.setProtocol('tcp')"
              :class="[
                'px-4 py-3 rounded-lg border-2 font-semibold transition-all',
                settingsStore.protocol === 'tcp'
                  ? 'border-blue-500 bg-blue-50 dark:bg-blue-500/10 text-blue-700 dark:text-blue-400'
                  : 'border-slate-300 dark:border-slate-700 text-slate-700 dark:text-slate-300 hover:border-slate-400'
              ]">
              TCP
            </button>
          </div>

          <p class="mt-3 p-3 rounded bg-slate-100 dark:bg-slate-800 text-xs text-slate-700 dark:text-slate-300">
            💡 <strong>Tip:</strong>
            <span v-if="settingsStore.protocol === 'udp'"> UDP is faster but may be blocked by restrictive networks. </span>
            <span v-else> TCP is slower but works better on restricted networks. </span>
          </p>
        </div>

        <hr class="border-slate-200 dark:border-slate-700" />

        <!-- 2. Obfuscation -->
        <div>
          <div class="flex items-center justify-between mb-4">
            <div>
              <h2 class="text-lg font-semibold">Random Obfuscation</h2>
              <p class="text-sm text-slate-600 dark:text-slate-400 mt-1">
                Disguise traffic to evade detection
              </p>
            </div>
            <div class="flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" :class="settingsStore.obfuscate ? 'text-emerald-500' : 'text-slate-400'">
                <path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10" />
                <g v-if="settingsStore.obfuscate">
                  <path d="m9 12 2 2 4-4" />
                </g>
              </svg>
            </div>
          </div>

          <button
            @click="settingsStore.toggleObfuscate()"
            :class="[
              'w-full px-4 py-3 rounded-lg border-2 font-semibold transition-all',
              settingsStore.obfuscate
                ? 'border-emerald-500 bg-emerald-50 dark:bg-emerald-500/10 text-emerald-700 dark:text-emerald-400'
                : 'border-slate-300 dark:border-slate-700 text-slate-700 dark:text-slate-300 hover:border-slate-400'
            ]">
            {{ settingsStore.obfuscate ? '✓ Obfuscation Enabled' : '✗ Obfuscation Disabled' }}
          </button>

          <!-- Obfuscation Key (if enabled) -->
          <div v-if="settingsStore.obfuscate" class="mt-3">
            <label class="block text-sm font-medium mb-2">Obfuscation Key (optional)</label>
            <input
              :value="settingsStore.obfuscateKey"
              @input="(e) => settingsStore.setObfuscateKey((e.target as HTMLInputElement).value)"
              type="text"
              placeholder="Leave empty for random key"
              class="w-full px-4 py-2 rounded-lg border border-slate-300 dark:border-slate-600 bg-white dark:bg-slate-800 focus:outline-none focus:ring-2 focus:ring-emerald-500 text-sm" />
            <p class="mt-2 text-xs text-slate-500 dark:text-slate-400">
              🔑 Leave blank to use a random key. Custom keys must be 32 characters.
            </p>
          </div>

          <p class="mt-3 p-3 rounded bg-slate-100 dark:bg-slate-800 text-xs text-slate-700 dark:text-slate-300">
            ⚠️ <strong>Note:</strong> Obfuscation adds minimal overhead but can help bypass DPI detection on some networks.
          </p>
        </div>

        <hr class="border-slate-200 dark:border-slate-700" />

        <!-- 3. Proxy Mode -->
        <div>
          <div class="flex items-center justify-between mb-4">
            <div>
              <h2 class="text-lg font-semibold">Proxy Mode</h2>
              <p class="text-sm text-slate-600 dark:text-slate-400 mt-1">
                Control how traffic is routed through the VPN
              </p>
            </div>
            <div class="flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="text-purple-500">
                <path d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2z"></path>
              </svg>
            </div>
          </div>

          <div class="space-y-3">
            <!-- Split Mode -->
            <label class="relative group cursor-pointer">
              <input
                type="radio"
                :value="'split'"
                v-model="settingsStore.proxyMode"
                @change="settingsStore.setProxyMode('split')"
                class="hidden" />
              <div :class="[
                'p-4 rounded-lg border-2 transition-all',
                settingsStore.proxyMode === 'split'
                  ? 'border-purple-500 bg-purple-50 dark:bg-purple-500/10'
                  : 'border-slate-300 dark:border-slate-700 group-hover:border-slate-400'
              ]">
                <div class="flex items-center gap-3">
                  <div :class="[
                    'w-5 h-5 rounded-full border-2 flex items-center justify-center transition-all',
                    settingsStore.proxyMode === 'split'
                      ? 'border-purple-500 bg-purple-500'
                      : 'border-slate-400'
                  ]">
                    <svg v-if="settingsStore.proxyMode === 'split'" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="white">
                      <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" />
                    </svg>
                  </div>
                  <div>
                    <h3 class="font-semibold">Split Mode (Default)</h3>
                    <p class="text-sm text-slate-600 dark:text-slate-400">China IPs bypass VPN, others use VPN</p>
                  </div>
                </div>
              </div>
            </label>

            <!-- Global Mode -->
            <label class="relative group cursor-pointer">
              <input
                type="radio"
                :value="'global'"
                v-model="settingsStore.proxyMode"
                @change="settingsStore.setProxyMode('global')"
                class="hidden" />
              <div :class="[
                'p-4 rounded-lg border-2 transition-all',
                settingsStore.proxyMode === 'global'
                  ? 'border-purple-500 bg-purple-50 dark:bg-purple-500/10'
                  : 'border-slate-300 dark:border-slate-700 group-hover:border-slate-400'
              ]">
                <div class="flex items-center gap-3">
                  <div :class="[
                    'w-5 h-5 rounded-full border-2 flex items-center justify-center transition-all',
                    settingsStore.proxyMode === 'global'
                      ? 'border-purple-500 bg-purple-500'
                      : 'border-slate-400'
                  ]">
                    <svg v-if="settingsStore.proxyMode === 'global'" xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="white">
                      <path d="M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z" />
                    </svg>
                  </div>
                  <div>
                    <h3 class="font-semibold">Global Proxy Mode</h3>
                    <p class="text-sm text-slate-600 dark:text-slate-400">All traffic routes through VPN</p>
                  </div>
                </div>
              </div>
            </label>
          </div>

          <p class="mt-3 p-3 rounded bg-slate-100 dark:bg-slate-800 text-xs text-slate-700 dark:text-slate-300">
            ℹ️ <strong>Split Mode:</strong> Better performance, but may expose your location for China IPs.<br/>
            <strong>Global Mode:</strong> All traffic encrypted, maximum privacy but slower.
          </p>
        </div>

        <hr class="border-slate-200 dark:border-slate-700" />

        <!-- Reset Button -->
        <button
          @click="resetSettings"
          class="w-full px-4 py-2 rounded-lg border border-slate-300 dark:border-slate-600 text-slate-700 dark:text-slate-300 font-medium hover:bg-slate-100 dark:hover:bg-slate-800 transition-all">
          Reset to Defaults
        </button>

      </div>

      <!-- Current Settings Summary -->
      <div class="bg-blue-50 dark:bg-blue-500/10 border border-blue-200 dark:border-blue-500/30 rounded-lg p-4">
        <h3 class="font-semibold text-blue-900 dark:text-blue-300 mb-2">Current Configuration</h3>
        <div class="space-y-1 text-sm text-blue-800 dark:text-blue-200">
          <p>Protocol: <span class="font-mono font-semibold">{{ settingsStore.protocol.toUpperCase() }}</span></p>
          <p>Obfuscation: <span class="font-mono font-semibold">{{ settingsStore.obfuscate ? 'Enabled' : 'Disabled' }}</span></p>
          <p>Proxy Mode: <span class="font-mono font-semibold capitalize">{{ settingsStore.proxyMode }}</span></p>
        </div>
      </div>

    </div>
  </div>
</template>

<script setup lang="ts">
import { useWireGuardMode } from '@/composables/useWireGuardMode';

const { settingsStore } = useWireGuardMode();

async function saveSettings() {
  try {
    await settingsStore.saveSettings();
    // Show success notification (can be implemented with Toast)
    console.log('✅ Settings saved successfully');
  } catch (err) {
    console.error('Failed to save settings:', err);
  }
}

async function resetSettings() {
  if (confirm('Reset settings to defaults?')) {
    await settingsStore.resetSettings();
  }
}
</script>
