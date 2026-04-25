/**
 * Pinia Store - WireGuard State Management
 * Handles core WireGuard connection and mode state
 */

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { wireguardApi, type AppMode } from '@/services/wireguard-api';
import { useEndpointsStore } from './endpoints';

export type ConnectionStatus = 'disconnected' | 'connecting' | 'connected' | 'error';

export const useWireGuardStore = defineStore('wireguard', () => {
  // ========== State ==========
  const mode = ref<AppMode>('client');
  const status = ref<ConnectionStatus>('disconnected');
  const isInitialized = ref(false);
  const error = ref<string | null>(null);
  const gatewayEnabled = ref(false);

  // ========== Computed ==========
  const isConnected = computed(() => status.value === 'connected');
  const isConnecting = computed(() => status.value === 'connecting');

  // ========== Actions ==========

  /**
   * Initialize store: detect current mode from backend.
   *
   * In dev, you can short-circuit the backend call by setting
   * `VITE_FORCE_MODE=server` (or `client`) in the environment, e.g.:
   *
   *     VITE_FORCE_MODE=server npm run dev
   *
   * or by adding `VITE_FORCE_MODE=server` to `frontend/.env.development.local`.
   *
   * If `/api/mode` is unreachable in dev, we keep the default mode and continue
   * so the UI still renders without a backend.
   */
  async function initialize() {
    if (isInitialized.value) return;

    const forcedMode = import.meta.env.DEV
      ? (import.meta.env.VITE_FORCE_MODE as AppMode | undefined)
      : undefined;

    if (forcedMode === 'server' || forcedMode === 'client') {
      mode.value = forcedMode;
      isInitialized.value = true;
      console.log('✅ WireGuard store initialized with forced mode:', mode.value);
      return;
    }

    try {
      const detectedMode = await wireguardApi.getMode();
      mode.value = detectedMode;
      isInitialized.value = true;
      console.log('✅ WireGuard store initialized with mode:', mode.value);
    } catch (err) {
      if (import.meta.env.DEV) {
        console.warn(
          '⚠️ /api/mode unreachable, using default mode:',
          mode.value,
          '(set VITE_FORCE_MODE to override)'
        );
        isInitialized.value = true;
        return;
      }
      error.value = `Failed to initialize: ${err}`;
      console.error('❌ Failed to initialize WireGuard store:', err);
      throw err;
    }
  }

  /**
   * Update mode (for future use when mode switching is implemented)
   */
  function setMode(newMode: AppMode) {
    mode.value = newMode;
  }

  /**
   * Update connection status
   */
  function setStatus(newStatus: ConnectionStatus) {
    status.value = newStatus;
    if (newStatus !== 'error') {
      error.value = null;
    }
  }

  /**
   * Connect to VPN
   */
  async function connect(endpointId: string) {
    try {
      setStatus('connecting');
      await wireguardApi.connect(endpointId);
      setStatus('connected');
      console.log('✅ Connected to VPN');
    } catch (err) {
      setStatus('error');
      error.value = `Connection failed: ${err}`;
      console.error('❌ Connection failed:', err);
      throw err;
    }
  }

  /**
   * Disconnect from VPN
   */
  async function disconnect() {
    try {
      setStatus('connecting');
      await wireguardApi.disconnect();
      setStatus('disconnected');
      console.log('✅ Disconnected from VPN');
    } catch (err) {
      setStatus('error');
      error.value = `Disconnection failed: ${err}`;
      console.error('❌ Disconnection failed:', err);
      throw err;
    }
  }

  /**
   * Fetch current stats and status
   */
  async function fetchStats() {
    try {
      const stats = await wireguardApi.getStats();
      if (stats) {
        const endpointsStore = useEndpointsStore();
        gatewayEnabled.value = stats.gateway_enabled;
        
        // Ensure the status is lowercase to match the ConnectionStatus type ('connected', 'disconnected', etc.)
        status.value = (stats.status || 'disconnected').toLowerCase() as ConnectionStatus;
        
        // Synchronize selected ID with endpoints store
        if (stats.selected_id) {
          endpointsStore.selectedId = stats.selected_id;
        }
        
        return stats;
      }
    } catch (err) {
      console.error('❌ Failed to fetch stats:', err);
    }
    return null;
  }

  /**
   * Toggle LAN Gateway
   */
  async function toggleGateway() {
    try {
      if (gatewayEnabled.value) {
        await wireguardApi.disableGateway();
        gatewayEnabled.value = false;
      } else {
        await wireguardApi.enableGateway();
        gatewayEnabled.value = true;
      }
    } catch (err) {
      error.value = `Failed to toggle gateway: ${err}`;
      console.error('❌ Failed to toggle gateway:', err);
      throw err;
    }
  }

  /**
   * Clear error
   */
  function clearError() {
    error.value = null;
  }

  return {
    // State
    mode,
    status,
    isInitialized,
    error,

    // Computed
    isConnected,
    isConnecting,
    gatewayEnabled,

    // Actions
    initialize,
    setMode,
    setStatus,
    connect,
    disconnect,
    toggleGateway,
    fetchStats,
    clearError,
  };
});
