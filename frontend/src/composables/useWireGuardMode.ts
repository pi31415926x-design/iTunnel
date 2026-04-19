/**
 * Composable - useWireGuardMode
 * Shared logic for mode detection and handling
 */

import { useWireGuardStore } from '@/stores/wireguard';
import { useEndpointsStore } from '@/stores/endpoints';
import { useSettingsStore } from '@/stores/settings';

export function useWireGuardMode() {
  const wireguardStore = useWireGuardStore();
  const endpointsStore = useEndpointsStore();
  const settingsStore = useSettingsStore();

  /**
   * Initialize all stores and load data
   */
  async function initializeApp() {
    try {
      // Initialize WireGuard store first to determine mode
      await wireguardStore.initialize();

      // Load endpoints and settings
      await Promise.all([
        endpointsStore.fetchEndpoints(),
        settingsStore.loadSettings(),
      ]);

      console.log('✅ App initialized successfully');
      return true;
    } catch (err) {
      console.error('❌ Failed to initialize app:', err);
      return false;
    }
  }

  /**
   * Check if running in client mode
   */
  function isClientMode() {
    return wireguardStore.mode === 'client';
  }

  /**
   * Check if running in server mode
   */
  function isServerMode() {
    return wireguardStore.mode === 'server';
  }

  return {
    wireguardStore,
    endpointsStore,
    settingsStore,
    initializeApp,
    isClientMode,
    isServerMode,
  };
}
