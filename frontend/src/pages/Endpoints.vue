<template>
  <div class="h-full overflow-y-auto p-4 lg:p-6 text-slate-900 dark:text-slate-100 bg-slate-50/50 dark:bg-slate-950/50">
    <div class="max-w-6xl mx-auto space-y-6">

      <!-- Header -->
      <div class="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 class="text-3xl font-bold">Endpoints</h1>
          <p class="text-slate-600 dark:text-slate-400 mt-1">Manage and connect to your VPN nodes.</p>
        </div>
        <div class="flex flex-col gap-3 sm:flex-row sm:items-center">
          <div class="flex gap-2 w-full sm:w-auto">
            <input
              v-model="searchQuery"
              type="search"
              placeholder="Search node / IP"
              class="w-full sm:w-72 rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm text-slate-900 shadow-sm outline-none transition focus:border-red-400 focus:ring-2 focus:ring-red-200 dark:border-slate-700 dark:bg-slate-900 dark:text-slate-100 dark:focus:border-red-500 dark:focus:ring-red-500/20"
            />
            <button
              @click="openAddModal"
              class="inline-flex items-center justify-center rounded-lg bg-red-600 p-2 text-white shadow-sm shadow-red-500/20 transition-all hover:bg-red-700 hover:shadow-red-500/40 active:scale-95 shrink-0"
              title="Add Endpoint"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                <path d="M12 5v14M5 12h14" />
              </svg>
            </button>
          </div>
        </div>
      </div>

      <!-- Loading State -->
      <div v-if="endpointsStore.loading" class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div v-for="i in 4" :key="i" class="h-64 rounded-3xl bg-white dark:bg-slate-900 animate-pulse border border-slate-200 dark:border-slate-800"></div>
      </div>

      <!-- Empty State -->
      <div v-else-if="filteredEndpoints.length === 0" class="rounded-3xl border border-dashed border-slate-300 bg-white/80 p-12 text-center text-slate-500 shadow-sm dark:border-slate-700 dark:bg-slate-900/80 dark:text-slate-400">
        <div class="mb-4 inline-flex h-16 w-16 items-center justify-center rounded-full bg-slate-100 dark:bg-slate-800 text-slate-400">
          <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
        </div>
        <p class="text-lg font-medium">No endpoints found</p>
        <p class="mt-2 text-sm max-w-xs mx-auto text-slate-400">Add your first custom endpoint to start using the tunnel.</p>
        <button @click="openAddModal" class="mt-6 font-semibold text-red-600 hover:text-red-700 transition">Add Endpoint &rarr;</button>
      </div>

      <!-- Endpoints Grid -->
      <div v-else class="grid grid-cols-1 md:grid-cols-2 gap-6">
        <div
          v-for="endpoint in filteredEndpoints"
          :key="endpoint.id"
          class="flex flex-col rounded-3xl border bg-white dark:bg-slate-900 transition-all duration-300 overflow-hidden relative group"
          :class="[
            endpointsStore.selectedId === endpoint.id 
              ? 'ring-2 ring-red-500/50 border-red-500 shadow-xl shadow-red-500/10' 
              : 'border-slate-200 dark:border-slate-800 hover:border-slate-300 dark:hover:border-slate-700 shadow-sm'
          ]"
        >
          <!-- Card Header -->
          <div class="px-5 py-4 flex items-center justify-between border-b border-slate-50 dark:border-slate-800/50">
            <div class="flex items-center gap-3 overflow-hidden">
              <span class="text-2xl shrink-0">{{ getEndpointIcon(endpoint) }}</span>
              <div class="min-w-0">
                <h3 class="font-bold text-lg truncate leading-tight">{{ getEndpointName(endpoint) }}</h3>
                <p class="text-[10px] text-slate-400 font-mono tracking-wider uppercase mt-0.5">ID: {{ endpoint.id }}</p>
              </div>
            </div>
            
            <div class="flex items-center gap-2">
              <button 
                @click.stop="handleConnect(endpoint)"
                class="h-9 w-9 flex items-center justify-center rounded-full transition-all active:scale-90"
                :class="[
                  wireguardStore.isConnected && endpointsStore.selectedId === endpoint.id
                    ? 'bg-red-50 text-red-600 hover:bg-red-600 hover:text-white'
                    : 'bg-red-50 text-red-600 hover:bg-red-600 hover:text-white'
                ]"
                :title="wireguardStore.isConnected && endpointsStore.selectedId === endpoint.id ? 'Disconnect' : 'Connect'"
              >
                <svg v-if="wireguardStore.isConnected && endpointsStore.selectedId === endpoint.id" xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M18.36 6.64a9 9 0 1 1-12.73 0"/><line x1="12" y1="2" x2="12" y2="12"/></svg>
                <svg v-else xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="currentColor" stroke="currentColor" stroke-width="2"><path d="m10 15 5-3-5-3v6Z"/><path d="M12 2a10 10 0 1 0 10 10A10 10 0 0 0 12 2Zm0 18a8 8 0 1 1 8-8 8 8 0 0 1-8 8Z"/></svg>
              </button>
              <button 
                @click.stop="handleEditEndpoint(endpoint)"
                class="h-9 w-9 flex items-center justify-center rounded-full bg-red-50 text-red-600 hover:bg-red-600 hover:text-white transition-all active:scale-90"
                title="Edit"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M11 4H4a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h14a2 2 0 0 0 2-2v-7"/><path d="M18.5 2.5a2.121 2.121 0 0 1 3 3L12 15l-4 1 1-4 9.5-9.5z"/></svg>
              </button>
              <button 
                @click.stop="handleDeleteEndpoint(endpoint.id as string)"
                class="h-9 w-9 flex items-center justify-center rounded-full bg-red-50 text-red-600 hover:bg-red-600 hover:text-white transition-all active:scale-90"
                title="Delete"
              >
                <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2"/></svg>
              </button>
            </div>
          </div>

          <!-- Card Content Grid -->
          <div class="p-5 grid grid-cols-2 gap-x-6 gap-y-4 text-sm bg-slate-50/50 dark:bg-slate-800/20 flex-1">
            <!-- Interface Info -->
            <div class="space-y-3">
              <h4 class="text-[10px] font-bold text-slate-500 uppercase tracking-widest flex items-center gap-1.5 border-b border-slate-200/50 dark:border-slate-700/50 pb-1">
                <span class="w-1.5 h-1.5 rounded-full bg-red-500"></span> Interface
              </h4>
              <div class="space-y-2">
                <div>
                  <label class="text-[10px] text-slate-400 block">PrivateKey</label>
                  <p class="font-mono text-xs truncate" :title="endpoint.wg_config?.interface.private_key">
                    {{ maskKey(endpoint.wg_config?.interface.private_key) || '—' }}
                  </p>
                </div>
                <div>
                  <label class="text-[10px] text-slate-400 block">IP Address</label>
                  <p class="font-semibold text-slate-700 dark:text-slate-200">{{ endpoint.wg_config?.interface.address || '—' }}</p>
                </div>
                <div class="flex gap-4">
                  <div class="flex-1">
                    <label class="text-[10px] text-slate-400 block">DNS</label>
                    <p class="text-[11px] truncate">{{ endpoint.wg_config?.interface.dns?.join(', ') || '—' }}</p>
                  </div>
                  <div class="w-12">
                    <label class="text-[10px] text-slate-400 block">MTU</label>
                    <p class="text-[11px]">{{ endpoint.wg_config?.interface.mtu || '1420' }}</p>
                  </div>
                </div>
              </div>
            </div>

            <!-- Peer Info -->
            <div class="space-y-3">
              <h4 class="text-[10px] font-bold text-slate-500 uppercase tracking-widest flex items-center gap-1.5 border-b border-slate-200/50 dark:border-slate-700/50 pb-1">
                <span class="w-1.5 h-1.5 rounded-full bg-red-500"></span> Peer
              </h4>
              <div class="space-y-2">
                <div>
                  <label class="text-[10px] text-slate-400 block">PublicKey</label>
                  <p class="font-mono text-xs truncate" :title="endpoint.wg_config?.peers[0]?.public_key">
                    {{ maskKey(endpoint.wg_config?.peers[0]?.public_key) || '—' }}
                  </p>
                </div>
                <div>
                  <label class="text-[10px] text-slate-400 block">PresharedKey</label>
                  <p class="font-mono text-xs truncate" :title="endpoint.wg_config?.peers[0]?.preshared_key">
                    {{ maskKey(endpoint.wg_config?.peers[0]?.preshared_key) || '—' }}
                  </p>
                </div>
                <div>
                  <label class="text-[10px] text-slate-400 block">Endpoint Port</label>
                  <p class="font-semibold text-red-600 dark:text-red-400">{{ endpoint.wg_config?.peers[0]?.endpoint || '—' }}</p>
                </div>
                <div>
                  <label class="text-[10px] text-slate-400 block">Allowed IPs</label>
                  <p class="text-[10px] line-clamp-1 text-slate-500" :title="endpoint.wg_config?.peers[0]?.allowed_ips?.join(', ')">
                    {{ endpoint.wg_config?.peers[0]?.allowed_ips?.join(', ') || '—' }}
                  </p>
                </div>
              </div>
            </div>
          </div>
          
          <!-- Selected Indicator -->
          <div v-if="endpointsStore.selectedId === endpoint.id" class="absolute top-2 left-2 flex h-5 w-5 items-center justify-center rounded-full bg-red-500 text-white shadow-lg">
            <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12"/></svg>
          </div>
        </div>
      </div>
    </div>

    <!-- Add/Edit Endpoint Modal -->
    <Transition name="fade">
      <div v-if="showModal" class="fixed inset-0 z-50 flex items-center justify-center p-4 bg-slate-900/60 backdrop-blur-sm">
        <div 
          ref="modalRef"
          :style="modalStyle"
          class="bg-white dark:bg-slate-900 w-full rounded-3xl shadow-2xl overflow-hidden animate-in fade-in zoom-in duration-300 relative flex flex-col"
          :class="{ 'max-w-lg': !isResized }"
        >
          <!-- Resize Handle -->
          <div 
            @mousedown="startResize"
            class="absolute bottom-0 right-0 w-8 h-8 cursor-nwse-resize flex items-end justify-end p-1.5 z-10 group"
          >
            <svg class="w-4 h-4 text-slate-300 dark:text-slate-700 group-hover:text-red-500 transition-colors" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round">
              <line x1="22" y1="14" x2="14" y2="22" />
              <line x1="22" y1="18" x2="18" y2="22" />
            </svg>
          </div>

          <div class="px-6 py-6 border-b border-slate-100 dark:border-slate-800 flex items-center justify-between bg-slate-50/50 dark:bg-slate-800/50 shrink-0">
            <h3 class="text-xl font-bold">{{ editingId ? 'Update Endpoint' : 'Add Custom Endpoint' }}</h3>
            <button @click="closeModal" class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-200 transition-colors">
              <svg class="h-6 w-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          <form @submit.prevent="handleSubmit" class="p-6 space-y-4 flex-1 overflow-y-auto min-h-0">
            <div class="space-y-2">
              <label class="text-sm font-semibold text-slate-700 dark:text-slate-300">Node Location</label>
              <input
                v-model="modalData.location"
                type="text"
                placeholder="e.g. USA, JP, HongKong"
                required
                class="w-full rounded-xl border border-slate-200 bg-white px-4 py-2.5 text-sm outline-none transition focus:border-red-400 focus:ring-2 focus:ring-red-200 dark:border-slate-700 dark:bg-slate-800 shadow-sm"
              />
            </div>

            <div class="space-y-2 flex-1 flex flex-col min-h-0">
              <label class="text-sm font-semibold text-slate-700 dark:text-slate-300">WireGuard Configuration</label>
              <textarea
                v-model="modalData.config"
                placeholder="Paste your [Peer] or [Interface] config block here..."
                required
                class="w-full flex-1 rounded-xl border border-slate-200 bg-white px-4 py-3 text-[13px] font-mono outline-none transition focus:border-red-400 focus:ring-2 focus:ring-red-200 dark:border-slate-700 dark:bg-slate-800 resize-none min-h-[220px]"
              ></textarea>
            </div>

            <div v-if="errorMsg" class="p-3 text-sm text-red-600 bg-red-50 dark:bg-red-900/20 dark:text-red-400 rounded-lg flex items-center gap-2">
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
              {{ errorMsg }}
            </div>

            <div class="flex gap-3 pt-4 shrink-0">
              <button
                type="button"
                @click="closeModal"
                class="flex-1 py-3 px-4 border border-slate-200 dark:border-slate-700 rounded-xl text-sm font-semibold hover:bg-slate-50 dark:hover:bg-slate-800 transition"
              >
                Cancel
              </button>
              <button
                type="submit"
                :disabled="submitting"
                class="flex-1 py-3 px-4 bg-red-600 text-white rounded-xl text-sm font-semibold hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed transition shadow-sm shadow-red-500/20 hover:shadow-red-500/40"
              >
                {{ submitting ? 'Saving...' : (editingId ? 'Update Node' : 'Add Node') }}
              </button>
            </div>
          </form>
        </div>
      </div>
    </Transition>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from 'vue';
import { useWireGuardMode } from '@/composables/useWireGuardMode';
import { type RawEndpointInfo } from '@/services/wireguard-api';

const { wireguardStore, endpointsStore } = useWireGuardMode();

const searchQuery = ref('');
const showModal = ref(false);
const editingId = ref<string | null>(null);
const submitting = ref(false);
const errorMsg = ref('');

const modalData = ref({
  location: '',
  config: ''
});

// Modal Resize Logic
const modalRef = ref<HTMLElement | null>(null);
const modalWidth = ref<number | null>(null);
const modalHeight = ref<number | null>(null);
const isResizing = ref(false);
const isResized = ref(false);

const modalStyle = computed(() => {
  if (!modalWidth.value || !modalHeight.value) return {};
  return {
    width: `${modalWidth.value}px`,
    height: `${modalHeight.value}px`,
  };
});

function startResize(e: MouseEvent) {
  e.preventDefault();
  isResizing.value = true;
  isResized.value = true;
  
  if (!modalWidth.value || !modalHeight.value) {
    if (modalRef.value) {
      modalWidth.value = modalRef.value.offsetWidth;
      modalHeight.value = modalRef.value.offsetHeight;
    }
  }

  window.addEventListener('mousemove', doResize);
  window.addEventListener('mouseup', stopResize);
  document.body.style.cursor = 'nwse-resize';
}

function doResize(e: MouseEvent) {
  if (!isResizing.value || !modalRef.value) return;
  const rect = modalRef.value.getBoundingClientRect();
  modalWidth.value = Math.max(400, Math.min(e.clientX - rect.left, window.innerWidth - 40));
  modalHeight.value = Math.max(300, Math.min(e.clientY - rect.top, window.innerHeight - 40));
}

function stopResize() {
  isResizing.value = false;
  window.removeEventListener('mousemove', doResize);
  window.removeEventListener('mouseup', stopResize);
  document.body.style.cursor = '';
}

let statsInterval: any;

onMounted(() => {
  endpointsStore.fetchEndpoints();
  // Start polling for status/stats to keep the UI in sync with backend
  wireguardStore.fetchStats();
  statsInterval = setInterval(() => {
    wireguardStore.fetchStats();
  }, 2000);
});

onUnmounted(() => {
  if (statsInterval) clearInterval(statsInterval);
});

const filteredEndpoints = computed(() => {
  const allEndpoints = endpointsStore.endpoints || [];
  if (!searchQuery.value) return allEndpoints;
  const q = searchQuery.value.toLowerCase();
  return allEndpoints.filter((e) => {
    const name = getEndpointName(e).toLowerCase();
    const address = (e as any).address?.toLowerCase() || '';
    return name.includes(q) || address.includes(q);
  });
});

async function handleSubmit() {
  if (submitting.value) return;
  submitting.value = true;
  errorMsg.value = '';

  try {
    const { wireguardApi } = await import('@/services/wireguard-api');
    let res;
    if (editingId.value) {
      res = await wireguardApi.updateEndpoint(editingId.value, modalData.value.location, modalData.value.config);
    } else {
      res = await wireguardApi.addEndpoint(modalData.value.location, modalData.value.config);
    }

    if (res.success) {
      await endpointsStore.fetchEndpoints();
      closeModal();
    } else {
      errorMsg.value = res.message || 'Operation failed';
    }
  } catch (err: any) {
    errorMsg.value = err.message || 'Connection error';
  } finally {
    submitting.value = false;
  }
}

async function handleConnect(endpoint: RawEndpointInfo) {
  try {
    const isCurrentActive = wireguardStore.isConnected && endpointsStore.selectedId === endpoint.id;
    
    if (isCurrentActive) {
      await wireguardStore.disconnect();
    } else if (endpoint.id) {
      await endpointsStore.selectEndpoint(endpoint.id);
      await wireguardStore.connect(endpoint.id);
    } else {
      console.error('Missing endpoint ID');
    }
  } catch (err) {
    console.error('Connection toggle failed:', err);
  }
}

function openAddModal() {
  editingId.value = null;
  modalData.value = { location: '', config: '' };
  showModal.value = true;
}

function handleEditEndpoint(endpoint: RawEndpointInfo) {
  editingId.value = endpoint.id as string;
  modalData.value = {
    location: getEndpointName(endpoint),
    config: generateConfigText(endpoint)
  };
  showModal.value = true;
}

async function handleDeleteEndpoint(id: string) {
  if (!confirm('Are you sure you want to delete this endpoint?')) return;
  
  try {
    const { wireguardApi } = await import('@/services/wireguard-api');
    const res = await wireguardApi.deleteEndpoint(id);
    if (res.success) {
      await endpointsStore.fetchEndpoints();
    }
  } catch (err) {
    console.error('Delete failed:', err);
  }
}

function closeModal() {
  showModal.value = false;
  editingId.value = null;
  modalData.value = { location: '', config: '' };
  errorMsg.value = '';
  isResized.value = false;
  modalWidth.value = null;
  modalHeight.value = null;
}

function maskKey(key: string | undefined): string {
  if (!key) return '';
  if (key.length <= 10) return key;
  return `${key.substring(0, 6)}...${key.substring(key.length - 4)}`;
}

function generateConfigText(e: RawEndpointInfo): string {
  if (!e.wg_config) return '';
  const iface = e.wg_config.interface;
  const peer = e.wg_config.peers[0];
  if (!peer) return '';
  
  let text = `[Interface]\nPrivateKey = ${iface.private_key}\nAddress = ${iface.address}\n`;
  if (iface.dns?.length) text += `DNS = ${iface.dns.join(', ')}\n`;
  if (iface.mtu) text += `MTU = ${iface.mtu}\n`;
  
  text += `\n[Peer]\nPublicKey = ${peer.public_key}\n`;
  if (peer.preshared_key) text += `PresharedKey = ${peer.preshared_key}\n`;
  text += `AllowedIPs = ${peer.allowed_ips.join(', ')}\n`;
  if (peer.endpoint) text += `Endpoint = ${peer.endpoint}\n`;
  if (peer.persistent_keepalive) text += `PersistentKeepalive = ${peer.persistent_keepalive}\n`;
  
  return text;
}

function getEndpointName(e: RawEndpointInfo): string {
  if ('node_location' in e) return e.node_location || 'Unknown Node';
  if ('name' in e) return e.name || 'Unknown Node';
  return 'Unknown Node';
}

function getEndpointIcon(e: RawEndpointInfo): string {
  const name = getEndpointName(e);
  if (name.includes('🇯🇵') || name.toLowerCase().includes('jp')) return '🇯🇵';
  if (name.includes('🇭🇰') || name.toLowerCase().includes('hk')) return '🇭🇰';
  if (name.includes('🇺🇸') || name.toLowerCase().includes('usa')) return '🇺🇸';
  if (name.includes('🇬🇧') || name.toLowerCase().includes('uk')) return '🇬🇧';
  if (name.includes('🇰🇷') || name.toLowerCase().includes('kr')) return '🇰🇷';
  if (name.includes('🇮🇳') || name.toLowerCase().includes('in')) return '🇮🇳';
  return '🌍';
}
</script>
<style scoped>
.fade-enter-active, .fade-leave-active { transition: opacity 0.3s ease; }
.fade-enter-from, .fade-leave-to { opacity: 0; }

/* Custom scrollbar for better appearance */
::-webkit-scrollbar {
  width: 6px;
}
::-webkit-scrollbar-track {
  background: transparent;
}
::-webkit-scrollbar-thumb {
  background: #e2e8f0;
  border-radius: 10px;
}
.dark ::-webkit-scrollbar-thumb {
  background: #334155;
}
</style>
