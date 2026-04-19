/**
 * Pinia Store - Settings Management
 * Handles WireGuard enhance mode settings (TCP, obfuscate, proxy mode)
 */

import { defineStore } from 'pinia';
import { ref } from 'vue';
import { wireguardApi, type Protocol, type ProxyMode } from '@/services/wireguard-api';

export const useSettingsStore = defineStore('settings', () => {
  // ========== State ==========
  const protocol = ref<Protocol>('udp');
  const obfuscate = ref(false);
  const obfuscateKey = ref<string>('');
  const proxyMode = ref<ProxyMode>('split');
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const hasChanges = ref(false);

  // ========== Actions ==========

  /**
   * Load enhance mode settings from backend
   */
  async function loadSettings() {
    try {
      loading.value = true;
      error.value = null;

      const response = await wireguardApi.getEnhanceMode();
      const settings = response.enhance_mode;

      protocol.value = settings.protocol;
      obfuscate.value = settings.obfuscate;
      proxyMode.value = settings.proxy_mode;
      obfuscateKey.value = settings.obfuscate_key || '';
      hasChanges.value = false;

      console.log('✅ Settings loaded:', settings);
    } catch (err) {
      error.value = `Failed to load settings: ${err}`;
      console.error('❌ Failed to load settings:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  }

  /**
   * Save enhance mode settings to backend
   */
  async function saveSettings() {
    try {
      saving.value = true;
      error.value = null;

      const response = await wireguardApi.saveEnhanceMode({
        protocol: protocol.value,
        obfuscate: obfuscate.value,
        obfuscateKey: obfuscateKey.value || undefined,
        proxyMode: proxyMode.value,
      });

      hasChanges.value = false;
      console.log('✅ Settings saved:', response.enhance_mode);
    } catch (err) {
      error.value = `Failed to save settings: ${err}`;
      console.error('❌ Failed to save settings:', err);
      throw err;
    } finally {
      saving.value = false;
    }
  }

  /**
   * Update protocol (TCP/UDP)
   */
  function setProtocol(proto: Protocol) {
    if (protocol.value !== proto) {
      protocol.value = proto;
      hasChanges.value = true;
    }
  }

  /**
   * Toggle obfuscation
   */
  function toggleObfuscate() {
    obfuscate.value = !obfuscate.value;
    hasChanges.value = true;
  }

  /**
   * Set obfuscation key
   */
  function setObfuscateKey(key: string) {
    obfuscateKey.value = key;
    hasChanges.value = true;
  }

  /**
   * Update proxy mode (split/global)
   */
  function setProxyMode(mode: ProxyMode) {
    if (proxyMode.value !== mode) {
      proxyMode.value = mode;
      hasChanges.value = true;
    }
  }

  /**
   * Reset settings to last saved state
   */
  async function resetSettings() {
    await loadSettings();
    hasChanges.value = false;
  }

  /**
   * Clear error
   */
  function clearError() {
    error.value = null;
  }

  return {
    // State
    protocol,
    obfuscate,
    obfuscateKey,
    proxyMode,
    loading,
    saving,
    error,
    hasChanges,

    // Actions
    loadSettings,
    saveSettings,
    setProtocol,
    toggleObfuscate,
    setObfuscateKey,
    setProxyMode,
    resetSettings,
    clearError,
  };
});
