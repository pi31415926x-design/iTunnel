<template>
  <div class="h-full flex flex-col overflow-hidden bg-slate-50 dark:bg-slate-900">
    <!-- Success/Error Notification -->
    <Transition enter-active-class="transition ease-out duration-300" enter-from-class="translate-y-[-100%] opacity-0"
      enter-to-class="translate-y-0 opacity-100" leave-active-class="transition ease-in duration-200"
      leave-from-class="translate-y-0 opacity-100" leave-to-class="translate-y-[-100%] opacity-0">
      <div v-if="notification.show" :class="[
        'fixed top-4 left-1/2 transform -translate-x-1/2 z-50 px-6 py-4 rounded-lg shadow-lg flex items-center gap-3 min-w-[320px]',
        notification.type === 'success' ? 'bg-green-500 text-white' : 'bg-red-500 text-white'
      ]">
        <svg v-if="notification.type === 'success'" class="w-6 h-6" fill="none" viewBox="0 0 24 24"
          stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 13l4 4L19 7" />
        </svg>
        <svg v-else class="w-6 h-6" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
        <span class="font-medium">{{ notification.message }}</span>
      </div>
    </Transition>

    <!-- Fixed Header and Tabs Section -->
    <div class="p-6 pb-0 flex-shrink-0">
      <div class="mb-6 flex items-start justify-between">
        <div>
          <h1 class="text-2xl font-bold text-slate-900 dark:text-white">Settings</h1>
          <p class="text-slate-500 dark:text-slate-400 mt-1 text-sm">Manage your tunnel configuration and peer settings.
          </p>
        </div>
        <button @click="saveAllSettings" :disabled="isSaving" :class="[
          'bg-indigo-600 hover:bg-indigo-700 text-white font-medium py-2 px-6 rounded-lg transition-all duration-200 shadow-md hover:shadow-indigo-500/25 flex items-center justify-center gap-2 active:scale-[0.98] text-sm min-w-[100px]',
          isSaving ? 'opacity-75 cursor-not-allowed' : ''
        ]">
          <svg v-if="!isSaving" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2"
            stroke="currentColor" class="w-4 h-4">
            <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 12.75l6 6 9-13.5" />
          </svg>
          <svg v-else class="animate-spin w-4 h-4" xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24">
            <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4"></circle>
            <path class="opacity-75" fill="currentColor"
              d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z">
            </path>
          </svg>
          <span>{{ isSaving ? 'Connecting...' : 'Connect' }}</span>
        </button>
      </div>

      <!-- Tabs Navigation -->
      <div class="border-b border-slate-200 dark:border-slate-700">
        <nav class="flex space-x-8" aria-label="Tabs">
          <button v-for="tab in tabs" :key="tab.id" @click="activeTab = tab.id" :class="[
            activeTab === tab.id
              ? 'border-indigo-500 text-indigo-600 dark:text-indigo-400'
              : 'border-transparent text-slate-500 hover:text-slate-700 hover:border-slate-300 dark:text-slate-400 dark:hover:text-slate-300',
            'whitespace-nowrap py-4 px-1 border-b-2 font-medium text-sm transition-colors duration-200 flex items-center space-x-2'
          ]">
            <component :is="tab.icon" class="w-5 h-5" />
            <span>{{ tab.name }}</span>
            <span v-if="hasTabErrors(tab.id)" class="w-2 h-2 bg-red-500 rounded-full"></span>
          </button>
        </nav>
      </div>
    </div>

    <!-- Scrollable Content Area -->
    <div class="flex-1 overflow-y-auto p-6 custom-scrollbar">
      <div class="max-w-3xl mx-auto">
        <div v-if="activeTab === 'interface'" class="animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div
            class="bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700 p-6 shadow-sm space-y-6">
            <div class="flex items-center justify-between">
              <h2 class="text-lg font-semibold text-slate-900 dark:text-white flex items-center gap-2">
                <div class="w-1 h-5 bg-indigo-500 rounded-full"></div>
                Interface Settings
              </h2>
              <button @click="generatePrivateKey"
                class="text-sm text-indigo-600 dark:text-indigo-400 hover:text-indigo-700 dark:hover:text-indigo-300 font-medium flex items-center gap-1">
                <svg class="w-4 h-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                    d="M15 7a2 2 0 012 2m4 0a6 6 0 01-7.743 5.743L11 17H9v2H7v2H4a1 1 0 01-1-1v-2.586a1 1 0 01.293-.707l5.964-5.964A6 6 0 1121 9z" />
                </svg>
                Generate Key
              </button>
            </div>

            <form @submit.prevent="saveAllSettings" class="space-y-5">
              <!-- Address -->
              <div class="space-y-1.5">
                <label for="address"
                  class="text-sm font-medium text-slate-700 dark:text-slate-300 flex items-center gap-1">
                  Address
                  <span class="text-red-500">*</span>
                </label>
                <input id="address" v-model="form.address" @blur="validateField('address')" type="text"
                  placeholder="e.g. 10.0.0.1/24" :class="[
                    'w-full px-4 py-2.5 rounded-lg border transition-all duration-200 bg-slate-50 dark:bg-slate-900 text-slate-900 dark:text-white focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none',
                    errors.address ? 'border-red-500 bg-red-50 dark:bg-red-900/10' : 'border-slate-200 dark:border-slate-700'
                  ]" />
                <p v-if="errors.address" class="text-xs text-red-500 flex items-center gap-1">
                  <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd"
                      d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                      clip-rule="evenodd" />
                  </svg>
                  {{ errors.address }}
                </p>
                <p v-else class="text-xs text-slate-500 dark:text-slate-400">The IP address for this interface (e.g.,
                  10.0.0.1/24)</p>
              </div>

              <!-- ListenPort -->
              <div class="space-y-1.5">
                <label for="listenPort"
                  class="text-sm font-medium text-slate-700 dark:text-slate-300 flex items-center gap-1">
                  Listen Port
                  <span class="text-red-500">*</span>
                </label>
                <input id="listenPort" v-model.number="form.listenPort" @blur="validateField('listenPort')"
                  type="number" placeholder="51820" :class="[
                    'w-full px-4 py-2.5 rounded-lg border transition-all duration-200 bg-slate-50 dark:bg-slate-900 text-slate-900 dark:text-white focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none',
                    errors.listenPort ? 'border-red-500 bg-red-50 dark:bg-red-900/10' : 'border-slate-200 dark:border-slate-700'
                  ]" />
                <p v-if="errors.listenPort" class="text-xs text-red-500 flex items-center gap-1">
                  <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd"
                      d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                      clip-rule="evenodd" />
                  </svg>
                  {{ errors.listenPort }}
                </p>
                <p v-else class="text-xs text-slate-500 dark:text-slate-400">UDP port for WireGuard (1-65535, default:
                  51820)</p>
              </div>

              <!-- PrivateKey -->
              <div class="space-y-1.5">
                <label for="privateKey"
                  class="text-sm font-medium text-slate-700 dark:text-slate-300 flex items-center gap-1">
                  Private Key
                  <span class="text-red-500">*</span>
                </label>
                <div class="relative">
                  <input id="privateKey" v-model="form.privateKey" @blur="validateField('privateKey')"
                    :type="showPrivateKey ? 'text' : 'password'" placeholder="Base64 encoded private key" :class="[
                      'w-full px-4 py-2.5 pr-10 rounded-lg border transition-all duration-200 bg-slate-50 dark:bg-slate-900 text-slate-900 dark:text-white focus:ring-2 focus:ring-indigo-500/20 focus:border-indigo-500 outline-none font-mono text-sm',
                      errors.privateKey ? 'border-red-500 bg-red-50 dark:bg-red-900/10' : 'border-slate-200 dark:border-slate-700'
                    ]" />
                  <button type="button" @click="showPrivateKey = !showPrivateKey"
                    class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-300">
                    <svg v-if="showPrivateKey" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
                    </svg>
                    <svg v-else class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                    </svg>
                  </button>
                </div>
                <p v-if="errors.privateKey" class="text-xs text-red-500 flex items-center gap-1">
                  <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd"
                      d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                      clip-rule="evenodd" />
                  </svg>
                  {{ errors.privateKey }}
                </p>
                <p v-else class="text-xs text-slate-500 dark:text-slate-400">Base64 encoded private key (click "Generate
                  Key" to create one)</p>
              </div>

              <!-- Toggles -->
              <div class="pt-2">
                <label class="text-sm font-medium text-slate-700 dark:text-slate-300 block mb-3">Options</label>
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-3">
                  <label
                    v-for="toggle in [{ id: 'isTcp', label: 'TCP Mode', desc: 'Use TCP instead of UDP' }, { id: 'isServer', label: 'Server Mode', desc: 'Act as server' }, { id: 'isGlobal', label: 'Global Mode', desc: 'Route all traffic' }]"
                    :key="toggle.id"
                    class="group relative flex items-start space-x-3 cursor-pointer p-4 rounded-lg bg-slate-50/50 dark:bg-slate-900/50 border border-slate-200 dark:border-slate-700 transition-all hover:bg-slate-100 dark:hover:bg-slate-800/50 hover:border-indigo-300 dark:hover:border-indigo-600">
                    <input type="checkbox" v-model="(form as any)[toggle.id]"
                      class="mt-0.5 w-4 h-4 text-indigo-600 rounded border-slate-300 focus:ring-indigo-500 bg-white dark:bg-slate-900">
                    <div class="flex-1">
                      <span class="text-sm font-medium text-slate-700 dark:text-slate-300 block">{{ toggle.label
                      }}</span>
                      <span class="text-xs text-slate-500 dark:text-slate-400">{{ toggle.desc }}</span>
                    </div>
                  </label>
                </div>
              </div>
            </form>
          </div>
        </div>

        <div v-else-if="activeTab === 'peers'" class="animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div
            class="bg-white dark:bg-slate-800 rounded-xl border border-slate-200 dark:border-slate-700 p-6 shadow-sm space-y-6">
            <h2 class="text-lg font-semibold text-slate-900 dark:text-white flex items-center gap-2">
              <div class="w-1 h-5 bg-indigo-500 rounded-full"></div>
              Peer Management
            </h2>

            <form @submit.prevent="saveAllSettings" class="space-y-5">
              <div class="space-y-1.5">
                <label for="publicKey"
                  class="text-sm font-medium text-slate-700 dark:text-slate-300 flex items-center gap-1">
                  Public Key
                  <span class="text-red-500">*</span>
                </label>
                <input id="publicKey" v-model="peerForm.publicKey" @blur="validatePeerField('publicKey')" type="text"
                  placeholder="Peer's base64 encoded public key" :class="[
                    'w-full px-4 py-2.5 rounded-lg border transition-all bg-slate-50 dark:bg-slate-900 text-slate-900 dark:text-white focus:ring-2 focus:ring-indigo-500/20 outline-none font-mono text-sm',
                    peerErrors.publicKey ? 'border-red-500 bg-red-50 dark:bg-red-900/10' : 'border-slate-200 dark:border-slate-700'
                  ]" />
                <p v-if="peerErrors.publicKey" class="text-xs text-red-500 flex items-center gap-1">
                  <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd"
                      d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                      clip-rule="evenodd" />
                  </svg>
                  {{ peerErrors.publicKey }}
                </p>
                <p v-else class="text-xs text-slate-500 dark:text-slate-400">The peer's public key for authentication
                </p>
              </div>

              <div class="space-y-1.5">
                <label for="presharedKey" class="text-sm font-medium text-slate-700 dark:text-slate-300">
                  Preshared Key <span class="text-slate-400 text-xs">(Optional)</span>
                </label>
                <div class="relative">
                  <input id="presharedKey" v-model="peerForm.presharedKey" @blur="validatePeerField('presharedKey')"
                    :type="showPresharedKey ? 'text' : 'password'"
                    placeholder="Optional preshared key for additional security" :class="[
                      'w-full px-4 py-2.5 pr-10 rounded-lg border transition-all bg-slate-50 dark:bg-slate-900 text-slate-900 dark:text-white focus:ring-2 focus:ring-indigo-500/20 outline-none font-mono text-sm',
                      peerErrors.presharedKey ? 'border-red-500 bg-red-50 dark:bg-red-900/10' : 'border-slate-200 dark:border-slate-700'
                    ]" />
                  <button type="button" @click="showPresharedKey = !showPresharedKey"
                    class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600 dark:hover:text-slate-300">
                    <svg v-if="showPresharedKey" class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M13.875 18.825A10.05 10.05 0 0112 19c-4.478 0-8.268-2.943-9.543-7a9.97 9.97 0 011.563-3.029m5.858.908a3 3 0 114.243 4.243M9.878 9.878l4.242 4.242M9.88 9.88l-3.29-3.29m7.532 7.532l3.29 3.29M3 3l3.59 3.59m0 0A9.953 9.953 0 0112 5c4.478 0 8.268 2.943 9.543 7a10.025 10.025 0 01-4.132 5.411m0 0L21 21" />
                    </svg>
                    <svg v-else class="w-5 h-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                      <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2"
                        d="M2.458 12C3.732 7.943 7.523 5 12 5c4.478 0 8.268 2.943 9.542 7-1.274 4.057-5.064 7-9.542 7-4.477 0-8.268-2.943-9.542-7z" />
                    </svg>
                  </button>
                </div>
                <p v-if="peerErrors.presharedKey" class="text-xs text-red-500 flex items-center gap-1">
                  <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd"
                      d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                      clip-rule="evenodd" />
                  </svg>
                  {{ peerErrors.presharedKey }}
                </p>
                <p v-else class="text-xs text-slate-500 dark:text-slate-400">Additional layer of symmetric encryption
                  (recommended)</p>
              </div>

              <div class="space-y-1.5">
                <label for="allowedIPs"
                  class="text-sm font-medium text-slate-700 dark:text-slate-300 flex items-center gap-1">
                  Allowed IPs
                  <span class="text-red-500">*</span>
                </label>
                <input id="allowedIPs" v-model="peerForm.allowedIPs" @blur="validatePeerField('allowedIPs')" type="text"
                  placeholder="0.0.0.0/0, ::/0" :class="[
                    'w-full px-4 py-2.5 rounded-lg border transition-all bg-slate-50 dark:bg-slate-900 text-slate-900 dark:text-white focus:ring-2 focus:ring-indigo-500/20 outline-none',
                    peerErrors.allowedIPs ? 'border-red-500 bg-red-50 dark:bg-red-900/10' : 'border-slate-200 dark:border-slate-700'
                  ]" />
                <p v-if="peerErrors.allowedIPs" class="text-xs text-red-500 flex items-center gap-1">
                  <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                    <path fill-rule="evenodd"
                      d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                      clip-rule="evenodd" />
                  </svg>
                  {{ peerErrors.allowedIPs }}
                </p>
                <p v-else class="text-xs text-slate-500 dark:text-slate-400">Comma-separated CIDR ranges (e.g.,
                  0.0.0.0/0 for all traffic)</p>
              </div>

              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="space-y-1.5">
                  <label for="endpoint"
                    class="text-sm font-medium text-slate-700 dark:text-slate-300 flex items-center gap-1">
                    Endpoint
                    <span class="text-red-500">*</span>
                  </label>
                  <input id="endpoint" v-model="peerForm.endpoint" @blur="validatePeerField('endpoint')" type="text"
                    placeholder="1.2.3.4:51820" :class="[
                      'w-full px-4 py-2.5 rounded-lg border transition-all bg-slate-50 dark:bg-slate-900 text-slate-900 dark:text-white focus:ring-2 focus:ring-indigo-500/20 outline-none',
                      peerErrors.endpoint ? 'border-red-500 bg-red-50 dark:bg-red-900/10' : 'border-slate-200 dark:border-slate-700'
                    ]" />
                  <p v-if="peerErrors.endpoint" class="text-xs text-red-500 flex items-center gap-1">
                    <svg class="w-3 h-3" fill="currentColor" viewBox="0 0 20 20">
                      <path fill-rule="evenodd"
                        d="M18 10a8 8 0 11-16 0 8 8 0 0116 0zm-7 4a1 1 0 11-2 0 1 1 0 012 0zm-1-9a1 1 0 00-1 1v4a1 1 0 102 0V6a1 1 0 00-1-1z"
                        clip-rule="evenodd" />
                    </svg>
                    {{ peerErrors.endpoint }}
                  </p>
                  <p v-else class="text-xs text-slate-500 dark:text-slate-400">Peer's IP:Port</p>
                </div>
                <div class="flex items-end">
                  <label
                    class="flex items-start space-x-3 cursor-pointer p-4 rounded-lg bg-slate-50 dark:bg-slate-900/50 border border-slate-200 dark:border-slate-700 w-full hover:bg-slate-100 dark:hover:bg-slate-800/50 transition-colors">
                    <input type="checkbox" v-model="peerForm.isChangeRoute"
                      class="mt-0.5 w-4 h-4 text-indigo-600 rounded border-slate-300 focus:ring-indigo-500">
                    <div>
                      <span class="text-sm font-medium text-slate-700 dark:text-slate-300 block">Change Route</span>
                      <span class="text-xs text-slate-500 dark:text-slate-400">Modify routing table</span>
                    </div>
                  </label>
                </div>
              </div>
            </form>
          </div>
        </div>

        <div v-else-if="activeTab === 'others'" class="animate-in fade-in slide-in-from-bottom-2 duration-300">
          <div
            class="bg-white dark:bg-slate-800 rounded-2xl border border-slate-200 dark:border-slate-700 p-12 text-center shadow-sm">
            <div
              class="mx-auto w-16 h-16 bg-indigo-50 dark:bg-indigo-900/30 rounded-full flex items-center justify-center mb-6">
              <SparklesIcon class="h-8 w-8 text-indigo-600 dark:text-indigo-400" />
            </div>
            <h2 class="text-xl font-bold text-slate-900 dark:text-white">More Settings Coming Soon</h2>
            <p class="text-slate-500 dark:text-slate-400 mt-2 max-w-xs mx-auto">
              We're working on advanced features like auto-start and notification preferences.
            </p>
          </div>
        </div>
      </div>
    </div>

  </div>
</template>

<script setup lang="ts">
import { ref, reactive, watch, onMounted } from 'vue';
import {
  GlobeAltIcon,
  UsersIcon,
  AdjustmentsHorizontalIcon,
  SparklesIcon
} from '@heroicons/vue/24/outline';

const activeTab = ref(localStorage.getItem('itunnel_settings_active_tab') || 'interface');

watch(activeTab, (newVal) => {
  localStorage.setItem('itunnel_settings_active_tab', newVal);
});

const tabs = [
  { id: 'interface', name: 'Interface', icon: GlobeAltIcon },
  { id: 'peers', name: 'Peers', icon: UsersIcon },
  { id: 'others', name: 'Others', icon: AdjustmentsHorizontalIcon },
];

// New state for UI enhancements
const isSaving = ref(false);
const showPrivateKey = ref(false);
const showPresharedKey = ref(false);

const notification = reactive({
  show: false,
  type: 'success' as 'success' | 'error',
  message: ''
});

const form = reactive({
  address: '',
  listenPort: null as number | null,
  privateKey: '',
  isTcp: false,
  isServer: false,
  isGlobal: true,
});

const errors = reactive({
  address: '',
  listenPort: '',
  privateKey: '',
});

const peerForm = reactive({
  publicKey: '',
  presharedKey: '',
  allowedIPs: '',
  endpoint: '',
  isChangeRoute: false,
});

const peerErrors = reactive({
  publicKey: '',
  presharedKey: '',
  allowedIPs: '',
  endpoint: '',
});

// Persistence
const STORAGE_KEY_INTERFACE = 'itunnel_settings_interface';
const STORAGE_KEY_PEER = 'itunnel_settings_peer';

onMounted(() => {
  const savedInterface = localStorage.getItem(STORAGE_KEY_INTERFACE);
  if (savedInterface) {
    try {
      Object.assign(form, JSON.parse(savedInterface));
    } catch (e) {
      console.error('Failed to load interface settings', e);
    }
  }

  const savedPeer = localStorage.getItem(STORAGE_KEY_PEER);
  if (savedPeer) {
    try {
      Object.assign(peerForm, JSON.parse(savedPeer));
    } catch (e) {
      console.error('Failed to load peer settings', e);
    }
  }
});

watch(form, (newVal) => {
  localStorage.setItem(STORAGE_KEY_INTERFACE, JSON.stringify(newVal));
}, { deep: true });

watch(peerForm, (newVal) => {
  localStorage.setItem(STORAGE_KEY_PEER, JSON.stringify(newVal));
}, { deep: true });

// Validation functions
const validateAddress = (addr: string) => {
  const ipv4Pattern = /^(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}$/;
  const ipv6Pattern = /^(?:(?:[a-fA-F\d]{1,4}:){7}(?:[a-fA-F\d]{1,4}|:)|(?:[a-fA-F\d]{1,4}:){6}(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|:[a-fA-F\d]{1,4}|:)|(?:[a-fA-F\d]{1,4}:){5}(?::(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,2}|:)|(?:[a-fA-F\d]{1,4}:){4}(?:(?::[a-fA-F\d]{1,4}){0,1}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,3}|:)|(?:[a-fA-F\d]{1,4}:){3}(?:(?::[a-fA-F\d]{1,4}){0,2}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,4}|:)|(?:[a-fA-F\d]{1,4}:){2}(?:(?::[a-fA-F\d]{1,4}){0,3}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,5}|:)|(?:[a-fA-F\d]{1,4}:){1}(?:(?::[a-fA-F\d]{1,4}){0,4}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,6}|:)|(?::(?:(?::[a-fA-F\d]{1,4}){0,5}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,7}|:)))(?:%[0-9a-zA-Z]{1,})?$/;
  return ipv4Pattern.test(addr) || ipv6Pattern.test(addr);
};

const validateBase64 = (str: string) => {
  const base64Pattern = /^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/;
  return base64Pattern.test(str);
};

const validateCIDR = (cidr: string) => {
  const ipv4CidrPattern = /^(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}\/(?:[0-9]|[1-2][0-9]|3[0-2])$/;
  const ipv6CidrPattern = /^(?:(?:[a-fA-F\d]{1,4}:){7}(?:[a-fA-F\d]{1,4}|:)|(?:[a-fA-F\d]{1,4}:){6}(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|:[a-fA-F\d]{1,4}|:)|(?:[a-fA-F\d]{1,4}:){5}(?::(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,2}|:)|(?:[a-fA-F\d]{1,4}:){4}(?:(?::[a-fA-F\d]{1,4}){0,1}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,3}|:)|(?:[a-fA-F\d]{1,4}:){3}(?:(?::[a-fA-F\d]{1,4}){0,2}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,4}|:)|(?:[a-fA-F\d]{1,4}:){2}(?:(?::[a-fA-F\d]{1,4}){0,3}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,5}|:)|(?:[a-fA-F\d]{1,4}:){1}(?:(?::[a-fA-F\d]{1,4}){0,4}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,6}|:)|(?::(?:(?::[a-fA-F\d]{1,4}){0,5}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,7}|:)))(?:%[0-9a-zA-Z]{1,})?\/(?:[0-9]|[1-9][0-9]|1[0-1][0-9]|12[0-8])$/;
  return ipv4CidrPattern.test(cidr) || ipv6CidrPattern.test(cidr);
};

const validateEndpoint = (endpoint: string) => {
  const ipv4PortPattern = /^(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}:(?:[0-9]{1,4}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5])$/;
  const ipv6PortPattern = /^\[(?:(?:[a-fA-F\d]{1,4}:){7}(?:[a-fA-F\d]{1,4}|:)|(?:[a-fA-F\d]{1,4}:){6}(?:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|:[a-fA-F\d]{1,4}|:)|(?:[a-fA-F\d]{1,4}:){5}(?::(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,2}|:)|(?:[a-fA-F\d]{1,4}:){4}(?:(?::[a-fA-F\d]{1,4}){0,1}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,3}|:)|(?:[a-fA-F\d]{1,4}:){3}(?:(?::[a-fA-F\d]{1,4}){0,2}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,4}|:)|(?:[a-fA-F\d]{1,4}:){2}(?:(?::[a-fA-F\d]{1,4}){0,3}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,5}|:)|(?:[a-fA-F\d]{1,4}:){1}(?:(?::[a-fA-F\d]{1,4}){0,4}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,6}|:)|(?::(?:(?::[a-fA-F\d]{1,4}){0,5}:(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)(?:\.(?:25[0-5]|2[0-4]\d|1\d\d|[1-9]\d|\d)){3}|(?::[a-fA-F\d]{1,4}){1,7}|:)))(?:%[0-9a-zA-Z]{1,})?\]:(?:[0-9]{1,4}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5])$/;
  return ipv4PortPattern.test(endpoint) || ipv6PortPattern.test(endpoint);
};

// Real-time field validation
const validateField = (field: 'address' | 'listenPort' | 'privateKey') => {
  if (field === 'address') {
    if (!form.address) {
      errors.address = '';
    } else if (!validateAddress(form.address)) {
      errors.address = 'Invalid IPv4 or IPv6 address';
    } else {
      errors.address = '';
    }
  } else if (field === 'listenPort') {
    if (form.listenPort === null || form.listenPort === undefined) {
      errors.listenPort = '';
    } else if (isNaN(form.listenPort) || form.listenPort < 0 || form.listenPort > 65535) {
      errors.listenPort = 'Invalid port number (0-65535)';
    } else {
      errors.listenPort = '';
    }
  } else if (field === 'privateKey') {
    if (!form.privateKey) {
      errors.privateKey = '';
    } else if (!validateBase64(form.privateKey)) {
      errors.privateKey = 'Invalid Base64 string';
    } else {
      errors.privateKey = '';
    }
  }
};

const validatePeerField = (field: 'publicKey' | 'presharedKey' | 'allowedIPs' | 'endpoint') => {
  if (field === 'publicKey') {
    if (!peerForm.publicKey) {
      peerErrors.publicKey = '';
    } else if (!validateBase64(peerForm.publicKey)) {
      peerErrors.publicKey = 'Invalid Base64 string';
    } else {
      peerErrors.publicKey = '';
    }
  } else if (field === 'presharedKey') {
    if (!peerForm.presharedKey) {
      peerErrors.presharedKey = '';
    } else if (!validateBase64(peerForm.presharedKey)) {
      peerErrors.presharedKey = 'Invalid Base64 string';
    } else {
      peerErrors.presharedKey = '';
    }
  } else if (field === 'allowedIPs') {
    if (!peerForm.allowedIPs) {
      peerErrors.allowedIPs = '';
    } else {
      const ips = peerForm.allowedIPs.split(',').map(s => s.trim());
      let hasError = false;
      for (const ip of ips) {
        if (!validateCIDR(ip)) {
          peerErrors.allowedIPs = `Invalid CIDR: ${ip}`;
          hasError = true;
          break;
        }
      }
      if (!hasError) {
        peerErrors.allowedIPs = '';
      }
    }
  } else if (field === 'endpoint') {
    if (!peerForm.endpoint) {
      peerErrors.endpoint = '';
    } else if (!validateEndpoint(peerForm.endpoint)) {
      peerErrors.endpoint = 'Invalid Endpoint format (IP:Port)';
    } else {
      peerErrors.endpoint = '';
    }
  }
};

// Check if a tab has errors
const hasTabErrors = (tabId: string) => {
  if (tabId === 'interface') {
    return !!(errors.address || errors.listenPort || errors.privateKey);
  } else if (tabId === 'peers') {
    return !!(peerErrors.publicKey || peerErrors.presharedKey || peerErrors.allowedIPs || peerErrors.endpoint);
  }
  return false;
};

// Show notification
const showNotification = (type: 'success' | 'error', message: string) => {
  notification.type = type;
  notification.message = message;
  notification.show = true;
  setTimeout(() => {
    notification.show = false;
  }, 3000);
};

// Generate private key (mock implementation - replace with actual key generation)
const generatePrivateKey = () => {
  // This is a mock implementation. In production, you should generate a real WireGuard private key
  const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';
  let result = '';
  for (let i = 0; i < 43; i++) {
    result += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  result += '=';
  form.privateKey = result;
  showNotification('success', 'Private key generated successfully');
};

const saveAllSettings = async () => {
  isSaving.value = true;

  // Validate Interface Settings
  errors.address = '';
  errors.listenPort = '';
  errors.privateKey = '';

  let isInterfaceValid = true;

  if (!form.address) {
    errors.address = 'Address is required';
    isInterfaceValid = false;
  } else if (!validateAddress(form.address)) {
    errors.address = 'Invalid IPv4 or IPv6 address';
    isInterfaceValid = false;
  }

  if (form.listenPort === null || form.listenPort === undefined) {
    errors.listenPort = 'Listen Port is required';
    isInterfaceValid = false;
  } else if (isNaN(form.listenPort) || form.listenPort < 0 || form.listenPort > 65535) {
    errors.listenPort = 'Invalid port number (0-65535)';
    isInterfaceValid = false;
  }

  if (!form.privateKey) {
    errors.privateKey = 'Private Key is required';
    isInterfaceValid = false;
  } else if (!validateBase64(form.privateKey)) {
    errors.privateKey = 'Invalid Base64 string';
    isInterfaceValid = false;
  }

  // Validate Peer Settings
  peerErrors.publicKey = '';
  peerErrors.presharedKey = '';
  peerErrors.allowedIPs = '';
  peerErrors.endpoint = '';

  let isPeerValid = true;

  if (!peerForm.publicKey) {
    peerErrors.publicKey = 'Public Key is required';
    isPeerValid = false;
  } else if (!validateBase64(peerForm.publicKey)) {
    peerErrors.publicKey = 'Invalid Base64 string';
    isPeerValid = false;
  }

  if (peerForm.presharedKey && !validateBase64(peerForm.presharedKey)) {
    peerErrors.presharedKey = 'Invalid Base64 string';
    isPeerValid = false;
  }

  if (!peerForm.allowedIPs) {
    peerErrors.allowedIPs = 'Allowed IPs are required';
    isPeerValid = false;
  } else {
    const ips = peerForm.allowedIPs.split(',').map(s => s.trim());
    for (const ip of ips) {
      if (!validateCIDR(ip)) {
        peerErrors.allowedIPs = `Invalid CIDR: ${ip}`;
        isPeerValid = false;
        break;
      }
    }
  }

  if (!peerForm.endpoint) {
    peerErrors.endpoint = 'Endpoint is required';
    isPeerValid = false;
  } else if (!validateEndpoint(peerForm.endpoint)) {
    peerErrors.endpoint = 'Invalid Endpoint format (IP:Port)';
    isPeerValid = false;
  }

  if (isInterfaceValid && isPeerValid) {
    const payload = {
      interface: { ...form },
      peers: { ...peerForm }
    };

    try {
      const response = await fetch('/api/setwg', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(payload),
      });

      if (response.ok) {
        showNotification('success', 'Settings saved successfully!');
      } else {
        const errorText = await response.text();
        showNotification('error', `Failed to save: ${errorText}`);
      }
    } catch (error) {
      showNotification('error', 'Network error. Please try again.');
      console.error('Error saving settings:', error);
    }
  } else {
    // Switch to the tab with errors
    if (!isInterfaceValid) {
      activeTab.value = 'interface';
      showNotification('error', 'Please fix interface settings errors');
    } else if (!isPeerValid) {
      activeTab.value = 'peers';
      showNotification('error', 'Please fix peer settings errors');
    }
  }

  isSaving.value = false;
};
</script>
