/**
 * Pinia Store - Endpoint Management
 * Handles endpoint selection and management
 */

import { defineStore } from 'pinia';
import { ref, computed } from 'vue';
import { wireguardApi, type EndpointInfo, type RawEndpointInfo } from '@/services/wireguard-api';

export const useEndpointsStore = defineStore('endpoints', () => {
  // ========== State ==========
  const endpoints = ref<RawEndpointInfo[]>([]);
  const selectedId = ref<string | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const lastUpdated = ref<Date | null>(null);

  // ========== Computed ==========
  const selectedEndpoint = computed(() => {
    if (!selectedId.value) return null;
    return endpoints.value.find((e) => e.id === selectedId.value) ?? null;
  });

  const isLegacyEndpoint = (endpoint: RawEndpointInfo): endpoint is EndpointInfo => {
    return typeof (endpoint as EndpointInfo).from_subscription !== 'undefined';
  };

  const subscriptionEndpoints = computed(() => {
    return endpoints.value.filter((e): e is EndpointInfo => isLegacyEndpoint(e) && e.from_subscription);
  });

  const manualEndpoints = computed(() => {
    return endpoints.value.filter((e): e is EndpointInfo => isLegacyEndpoint(e) && !e.from_subscription);
  });

  // ========== Actions ==========

  /**
   * Fetch all endpoints from backend
   */
  async function fetchEndpoints() {
    try {
      loading.value = true;
      error.value = null;

      const response = await wireguardApi.getEndpoints();
      endpoints.value = response.endpoints || [];
      if (response.selected_id) {
        selectedId.value = response.selected_id;
      }
      lastUpdated.value = new Date();

      console.log('✅ Endpoints loaded:', endpoints.value.length);
    } catch (err) {
      error.value = `Failed to load endpoints: ${err}`;
      console.error('❌ Failed to load endpoints:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  }

  /**
   * Select an endpoint
   */
  async function selectEndpoint(id: string) {
    try {
      loading.value = true;
      await wireguardApi.selectEndpoint(id);
      selectedId.value = id;
      console.log('✅ Endpoint selected:', id);
    } catch (err) {
      error.value = `Failed to select endpoint: ${err}`;
      console.error('❌ Failed to select endpoint:', err);
      throw err;
    } finally {
      loading.value = false;
    }
  }

  /**
   * Add manual endpoint (for future use)
   */
  function addManualEndpoint(endpoint: EndpointInfo) {
    endpoint.from_subscription = false;
    endpoints.value.push(endpoint);
  }

  /**
   * Remove endpoint (for future use)
   */
  function removeEndpoint(id: string) {
    const index = endpoints.value.findIndex((e) => e.id === id);
    if (index > -1) {
      endpoints.value.splice(index, 1);
    }
    if (selectedId.value === id) {
      selectedId.value = null;
    }
  }

  /**
   * Update endpoint info (latency, etc.)
   */
  function updateEndpoint(id: string, updates: Partial<RawEndpointInfo>) {
    const endpoint = endpoints.value.find((e) => e.id === id);
    if (endpoint) {
      Object.assign(endpoint, updates);
    }
  }

  /**
   * Clear all endpoints
   */
  function clear() {
    endpoints.value = [];
    selectedId.value = null;
  }

  return {
    // State
    endpoints,
    selectedId,
    loading,
    error,
    lastUpdated,

    // Computed
    selectedEndpoint,
    subscriptionEndpoints,
    manualEndpoints,

    // Actions
    fetchEndpoints,
    selectEndpoint,
    addManualEndpoint,
    removeEndpoint,
    updateEndpoint,
    clear,
  };
});
