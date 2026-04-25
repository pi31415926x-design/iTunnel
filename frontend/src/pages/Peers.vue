<template>
  <div class="h-full overflow-y-auto p-4 lg:p-8 text-slate-900 bg-slate-50/50 dark:bg-slate-950/50 dark:text-slate-100">
    <div class="max-w-5xl mx-auto">
      
      <!-- Container -->
      <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-xl overflow-hidden shadow-sm">
        
        <!-- Header -->
        <div class="px-6 py-5 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between">
          <h2 class="text-2xl font-bold">Clients</h2>
          <button 
            @click="openAddPeerModal"
            class="inline-flex items-center gap-2 px-4 py-2 text-sm font-medium border border-slate-200 dark:border-slate-700 rounded-lg hover:bg-slate-50 dark:hover:bg-slate-800 transition-colors">
            <PlusIcon class="w-4 h-4" />
            New
          </button>
        </div>

        <!-- Clients List -->
        <div class="divide-y divide-slate-100 dark:divide-slate-800/60">
          <div v-for="client in clients" :key="client.id" class="px-6 py-5 flex items-center gap-4 transition-colors hover:bg-slate-50/50 dark:hover:bg-slate-800/30">
            
            <!-- Avatar -->
            <div class="relative shrink-0 cursor-pointer hover:opacity-80 transition-opacity" @click="editPeer(client)" title="Click to edit">
              <div class="w-12 h-12 rounded-full bg-slate-100 dark:bg-slate-800 flex items-center justify-center text-slate-400">
                <UserIcon class="w-6 h-6" stroke-width="2" />
              </div>
              <div v-if="client.active && client.enabled" class="absolute bottom-0 right-0 w-3.5 h-3.5 bg-white dark:bg-slate-900 rounded-full flex items-center justify-center">
                <div class="w-2.5 h-2.5 bg-red-600 rounded-full"></div>
              </div>
            </div>

            <!-- Info -->
            <div class="flex-1 min-w-0 pr-4">
              <h3 class="text-[1.05rem] font-medium truncate">{{ client.name || 'Unnamed Peer' }}</h3>
              <div class="mt-0.5 flex flex-wrap items-center gap-x-2 gap-y-1 text-sm text-slate-500 dark:text-slate-400 text-opacity-80">
                <span>{{ client.ip }}</span>
                <template v-if="client.active && client.enabled">
                  <span class="text-slate-300 dark:text-slate-600">·</span>
                  <span class="flex items-center gap-0.5">
                    <ArrowDownIcon class="w-3.5 h-3.5" /> {{ client.rxSpeed }}
                  </span>
                  <span class="flex items-center gap-0.5">
                    <ArrowUpIcon class="w-3.5 h-3.5" /> {{ client.txSpeed }}
                  </span>
                  <span class="text-slate-300 dark:text-slate-600">·</span>
                  <span>{{ client.lastSeen }}</span>
                </template>
              </div>
            </div>

            <!-- Actions -->
            <div class="flex items-center gap-3 shrink-0">
              
              <!-- Toggle -->
              <button 
                @click="client.enabled = !client.enabled"
                :class="[
                  client.enabled ? 'bg-red-700' : 'bg-slate-200 dark:bg-slate-700',
                  'relative inline-flex h-7 w-12 items-center rounded-full transition-colors mr-3'
                ]"
              >
                <span :class="[
                  client.enabled ? 'translate-x-6 bg-white' : 'translate-x-1 bg-white dark:bg-slate-400',
                  'inline-block h-5 w-5 transform rounded-full transition-transform shadow-sm'
                ]" />
              </button>

              <!-- Action Buttons -->
              <div class="flex items-center gap-1.5">
                <button @click="openQrModal(client)" class="w-9 h-9 rounded-lg bg-slate-100 dark:bg-slate-800 text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 flex items-center justify-center transition-colors hover:bg-slate-200 dark:hover:bg-slate-700">
                  <QrCodeIcon class="w-5 h-5" />
                </button>
                <button @click="handleDownload(client)" class="w-9 h-9 rounded-lg bg-slate-100 dark:bg-slate-800 text-slate-500 hover:text-slate-700 dark:hover:text-slate-300 flex items-center justify-center transition-colors hover:bg-slate-200 dark:hover:bg-slate-700">
                  <ArrowDownTrayIcon class="w-5 h-5" />
                </button>
                <button @click="deletePeer(client)" class="w-9 h-9 rounded-lg bg-slate-100 dark:bg-slate-800 text-slate-500 hover:text-red-500 dark:hover:text-red-400 flex items-center justify-center transition-colors">
                  <TrashIcon class="w-5 h-5" />
                </button>
              </div>

            </div>

          </div>
        </div>

      </div>
    </div>

    <!-- Add Peer Modal -->
    <div v-if="showModal" class="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <div 
        class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl shadow-2xl w-full max-w-lg overflow-hidden flex flex-col transform transition-all"
      >
        <!-- Modal Header -->
        <div class="px-6 py-5 border-b border-slate-200 dark:border-slate-800 flex items-center justify-between bg-slate-50/50 dark:bg-slate-900/50">
          <h3 class="text-xl font-bold flex items-center gap-2">
            <UserIcon class="w-5 h-5 text-red-600" />
            {{ isEdit ? 'Edit Peer' : 'Add new peer' }}
          </h3>
          <button @click="showModal = false" class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-300 transition-colors p-1 rounded-full hover:bg-slate-100 dark:hover:bg-slate-800">
            <XMarkIcon class="w-5 h-5" />
          </button>
        </div>

        <!-- Modal Body -->
        <div class="p-6 space-y-5 overflow-y-auto max-h-[70vh]">
          
          <!-- Name -->
          <div>
            <label class="block text-sm font-medium text-slate-700 dark:text-slate-300 mb-1.5">Peer Name</label>
            <input 
              v-model="formData.name" 
              type="text" 
              placeholder="e.g. My Phone" 
              class="w-full px-3.5 py-2.5 bg-white dark:bg-slate-900 border border-slate-300 dark:border-slate-700 rounded-xl focus:outline-none focus:ring-2 focus:ring-red-500/50 focus:border-red-500 transition-shadow" 
            />
          </div>

          <!-- Key Generation Area -->
          <div class="p-4 bg-slate-50 dark:bg-slate-800/50 rounded-xl border border-slate-100 dark:border-slate-800 space-y-4">
            
            <div class="flex items-center justify-between mb-2">
              <h4 class="text-sm font-semibold tracking-wide text-slate-500 dark:text-slate-400 uppercase">WireGuard Keys</h4>
              <button 
                @click="generateKeys" 
                class="flex items-center gap-1.5 text-xs font-medium text-red-600 hover:text-red-700 transition-colors"
                title="Regenerate Keys"
              >
                <ArrowPathIcon class="w-3.5 h-3.5" :class="{ 'animate-spin': isGenerating }" />
                Regenerate
              </button>
            </div>

            <!-- Private Key -->
            <div>
              <label class="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1.5">Private Key</label>
              <input 
                v-model="formData.private_key" 
                type="text" 
                readonly
                class="w-full px-3 py-2 bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-700 rounded-lg text-sm font-mono text-slate-600 dark:text-slate-400 focus:outline-none" 
              />
            </div>
            
            <!-- Public Key -->
            <div>
              <label class="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1.5">Public Key</label>
              <input 
                v-model="formData.public_key" 
                type="text" 
                readonly
                class="w-full px-3 py-2 bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-700 rounded-lg text-sm font-mono text-slate-600 dark:text-slate-400 focus:outline-none" 
              />
            </div>
            
            <!-- Preshared Key -->
            <div>
              <label class="block text-xs font-medium text-slate-700 dark:text-slate-300 mb-1.5">Preshared Key</label>
              <input 
                v-model="formData.preshared_key" 
                type="text" 
                readonly
                class="w-full px-3 py-2 bg-white dark:bg-slate-950 border border-slate-200 dark:border-slate-700 rounded-lg text-sm font-mono text-slate-600 dark:text-slate-400 focus:outline-none" 
              />
            </div>
          </div>

          <!-- Network Configurations -->
          <div class="space-y-4">
            <!-- Allowed IPs -->
            <div>
              <label class="block text-sm font-medium text-slate-700 dark:text-slate-300 mb-1.5">Allowed IPs</label>
              <input 
                v-model="formData.allowed_ips" 
                type="text" 
                placeholder="e.g. 10.99.0.2/32" 
                class="w-full px-3.5 py-2.5 bg-white dark:bg-slate-900 border border-slate-300 dark:border-slate-700 rounded-xl focus:outline-none focus:ring-2 focus:ring-red-500/50 focus:border-red-500 transition-shadow" 
              />
              <p class="mt-1 text-xs text-slate-500">The IP address this peer will use within the VPN.</p>
            </div>
            
            <!-- Endpoint -->
            <div>
              <label class="block text-sm font-medium text-slate-700 dark:text-slate-300 mb-1.5">Endpoint (Optional)</label>
              <input 
                v-model="formData.endpoint" 
                type="text" 
                placeholder="e.g. 192.168.1.100:51820" 
                class="w-full px-3.5 py-2.5 bg-white dark:bg-slate-900 border border-slate-300 dark:border-slate-700 rounded-xl focus:outline-none focus:ring-2 focus:ring-red-500/50 focus:border-red-500 transition-shadow" 
              />
              <p class="mt-1 text-xs text-slate-500">Leave blank if this peer roams (Dynamic IP).</p>
            </div>
          </div>

        </div>

        <!-- Modal Footer -->
        <div class="px-6 py-5 border-t border-slate-200 dark:border-slate-800 bg-slate-50/80 dark:bg-slate-900 flex justify-end gap-3 rounded-b-2xl">
          <button 
            @click="showModal = false" 
            class="px-5 py-2 text-sm font-medium text-slate-600 dark:text-slate-300 hover:text-slate-900 dark:hover:text-white hover:bg-slate-200 dark:hover:bg-slate-800 rounded-lg transition-colors"
          >
            Cancel
          </button>
          <button 
            @click="savePeer" 
            :disabled="isSaving" 
            class="inline-flex items-center gap-2 px-6 py-2 text-sm font-medium text-white bg-red-600 hover:bg-red-700 disabled:opacity-50 disabled:cursor-not-allowed rounded-lg shadow-sm shadow-red-500/20 transition-all hover:shadow-red-500/40"
          >
            <ArrowPathIcon v-if="isSaving" class="w-4 h-4 animate-spin" />
            <span v-if="isSaving">Saving...</span>
            <span v-else>{{ isEdit ? 'Save Changes' : 'Add Peer' }}</span>
          </button>
        </div>
      </div>
    </div>

    <!-- QR Code Modal -->
    <div v-if="showQrModal" class="fixed inset-0 bg-slate-900/40 backdrop-blur-sm z-50 flex items-center justify-center p-4">
      <div class="bg-white dark:bg-slate-900 border border-slate-200 dark:border-slate-800 rounded-2xl shadow-xl w-full max-w-sm overflow-hidden flex flex-col transform transition-all text-center">
        <!-- Close Button -->
        <div class="px-5 py-4 flex items-center justify-between border-b border-slate-100 dark:border-slate-800/80 bg-slate-50/50 dark:bg-slate-900/50">
          <h3 class="text-base font-bold flex items-center gap-2">
            <QrCodeIcon class="w-5 h-5 text-slate-400" />
            Config: {{ activeClient?.name || 'Peer' }}
          </h3>
          <button @click="showQrModal = false" class="text-slate-400 hover:text-slate-600 dark:hover:text-slate-300 transition-colors p-1.5 rounded-full hover:bg-slate-200 dark:hover:bg-slate-800">
            <XMarkIcon class="w-5 h-5" />
          </button>
        </div>
        <!-- QR Code Body -->
        <div class="p-6 flex flex-col items-center">
          <div class="bg-white p-2.5 rounded-xl shadow-sm border border-slate-200 mb-5 inline-block">
            <img :src="'https://api.qrserver.com/v1/create-qr-code/?size=220x220&margin=1&data=' + encodeURIComponent(generatePeerConfigText(activeClient))" class="w-52 h-52 object-contain" alt="QR Code" />
          </div>
          <button 
             @click="handleDownload(activeClient)" 
             class="inline-flex items-center gap-2 px-6 py-2.5 text-sm font-medium text-slate-700 bg-slate-100 hover:bg-slate-200 dark:text-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700 rounded-xl transition-colors w-full justify-center shadow-sm">
            <ArrowDownTrayIcon class="w-4 h-4" />
            Download Config
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { 
  PlusIcon,
  UserIcon,
  ArrowDownIcon,
  ArrowUpIcon,
  QrCodeIcon,
  ArrowDownTrayIcon,
  TrashIcon,
  XMarkIcon,
  ArrowPathIcon
} from '@heroicons/vue/20/solid';

const clients = ref<any[]>([]);
const serverPublicKey = ref<string>('');
const serverEndpoint = ref<string>('');

const loadPeers = async () => {
  try {
    const res = await fetch('/api/peer_list');
    const json = await res.json();
    if (json.success && json.data) {
      serverPublicKey.value = json.server_public_key || '<MISSING_SERVER_PUBLIC_KEY>';
      serverEndpoint.value = json.server_endpoint || '<SERVER_ENDPOINT>';
      clients.value = json.data.map((peer: any, idx: number) => ({
        id: idx.toString(),
        name: peer.name || 'Unnamed Peer',
        ip: peer.allowed_ips.length > 0 ? peer.allowed_ips[0] : 'N/A',
        rxSpeed: '',
        txSpeed: '',
        lastSeen: 'just now',
        enabled: true,
        active: false,
        rawQuery: peer
      }));
    }
  } catch (e) {
    console.error('Failed to load peers', e);
  }
};

onMounted(() => {
  loadPeers();
});

const showModal = ref(false);
const showQrModal = ref(false);
const activeClient = ref<any>(null);
const isGenerating = ref(false);
const isSaving = ref(false);
const isEdit = ref(false);
const originalPublicKey = ref('');

const formData = ref({
  name: '',
  private_key: '',
  public_key: '',
  preshared_key: '',
  allowed_ips: '',
  endpoint: '',
});

const openAddPeerModal = async () => {
  isEdit.value = false;
  originalPublicKey.value = '';
  showModal.value = true;
  // Initialize fields
  formData.value.name = '';
  formData.value.private_key = '';
  formData.value.public_key = '';
  formData.value.preshared_key = '';
  formData.value.allowed_ips = '';
  formData.value.endpoint = '';
  // Auto-generate keys and IP
  await generateKeys();
};

const editPeer = (client: any) => {
  isEdit.value = true;
  originalPublicKey.value = client.rawQuery.public_key;
  
  formData.value.name = client.rawQuery.name || '';
  formData.value.private_key = client.rawQuery.private_key || '';
  formData.value.public_key = client.rawQuery.public_key || '';
  formData.value.preshared_key = client.rawQuery.preshared_key || '';
  formData.value.allowed_ips = client.rawQuery.allowed_ips ? client.rawQuery.allowed_ips.join(', ') : '';
  formData.value.endpoint = client.rawQuery.endpoint || '';

  showModal.value = true;
};

const deletePeer = async (client: any) => {
  if (!confirm(`Are you sure you want to delete peer ${client.name || client.public_key}?`)) return;
  
  try {
    const res = await fetch('/api/peer_delete', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ public_key: client.rawQuery.public_key })
    });
    
    const json = await res.json();
    if (json.success) {
      await loadPeers();
    } else {
      alert("Error deleting peer: " + json.message);
    }
  } catch (e) {
    console.error('Failed to delete peer', e);
  }
};

const openQrModal = (client: any) => {
  activeClient.value = client;
  showQrModal.value = true;
};

const generatePeerConfigText = (client: any) => {
  if (!client) return '';
  return `[Interface]
PrivateKey = ${client.rawQuery?.private_key || '<MISSING_PRIVATE_KEY>'}
Address = ${client.ip || '10.99.0.x/32'}
DNS = 1.1.1.1

[Peer]
PublicKey = ${serverPublicKey.value}
${client.rawQuery?.preshared_key ? 'PresharedKey = ' + client.rawQuery.preshared_key + '\n' : ''}Endpoint = ${serverEndpoint.value}
AllowedIPs = 0.0.0.0/0, ::/0
PersistentKeepalive = 25`;
};

const handleDownload = (client: any) => {
  if (!client) return;
  const text = generatePeerConfigText(client);
  const element = document.createElement('a');
  element.setAttribute('href', 'data:text/plain;charset=utf-8,' + encodeURIComponent(text));
  element.setAttribute('download', `${client.name || 'peer'}.conf`);
  element.style.display = 'none';
  document.body.appendChild(element);
  element.click();
  document.body.removeChild(element);
};

const generateKeys = async () => {
  if (isGenerating.value) return;
  isGenerating.value = true;
  try {
    const res = await fetch('/api/peer_generate');
    const json = await res.json();
    if (json.success && json.data) {
      formData.value.private_key = json.data.private_key;
      formData.value.public_key = json.data.public_key;
      formData.value.preshared_key = json.data.preshared_key;
      formData.value.allowed_ips = json.data.recommended_ip || '';
    }
  } catch (e) {
    console.error('Failed to generate keys', e);
  } finally {
    isGenerating.value = false;
  }
};

const savePeer = async () => {
  if (!formData.value.public_key || !formData.value.allowed_ips || !formData.value.name) {
    alert("Name, Public key and Allowed IPs are required!");
    return;
  }

  isSaving.value = true;
  try {
    const apiPath = isEdit.value ? '/api/peer_update' : '/api/peer_add';
    const payload: any = {
      name: formData.value.name,
      private_key: formData.value.private_key,
      public_key: formData.value.public_key,
      preshared_key: formData.value.preshared_key,
      allowed_ips: formData.value.allowed_ips,
      endpoint: formData.value.endpoint.trim() !== '' ? formData.value.endpoint : undefined, 
    };

    if (isEdit.value) {
      payload.original_public_key = originalPublicKey.value;
    }

    const res = await fetch(apiPath, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });
    
    // We expect { success: true, message: ... }
    const json = await res.json();
    if (json.success) {
      showModal.value = false;
      // Reload peers list
      await loadPeers();
    } else {
      alert("Error: " + json.message);
    }
  } catch (e) {
    console.error('Failed to save peer', e);
    alert('Failed to save peer. Check backend logs.');
  } finally {
    isSaving.value = false;
  }
};
</script>
