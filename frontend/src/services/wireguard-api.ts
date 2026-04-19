/**
 * WireGuard API Service
 * Handles WireGuard-specific API calls
 */

import { apiClient } from './api';

export type AppMode = 'client' | 'server';
export type Protocol = 'udp' | 'tcp';
export type ProxyMode = 'split' | 'global';

export interface InterfaceConfig {
  private_key: string;
  listen_port: number;
  address: string;
  dns: string[];
  mtu: number;
}

export interface PeerConfig {
  public_key: string;
  preshared_key: string;
  allowed_ips: string[];
  endpoint: string;
  persistent_keepalive?: number;
}

export interface WgConfig {
  interface: InterfaceConfig;
  peers: PeerConfig[];
}

export interface EndpointInfo {
  id: string;
  name: string;
  address: string;
  port: number;
  location?: string;
  latency?: number;
  from_subscription: boolean;
  wg_config?: WgConfig;
}

export interface NodeEndpointInfo {
  id?: string;
  node_location?: string;
  ip4?: string;
  ip6?: string;
  active?: boolean;
  location?: string;
  latency?: number;
  from_subscription?: boolean;
  name?: string;
  address?: string;
  port?: number;
  wg_config?: WgConfig;
}

export type RawEndpointInfo = EndpointInfo | NodeEndpointInfo;

export interface EnhanceMode {
  protocol: Protocol;
  obfuscate: boolean;
  proxy_mode: ProxyMode;
  obfuscate_key?: string;
}

export interface ModeResponse {
  mode: AppMode;
  success: boolean;
}

export interface EndpointsResponse {
  endpoints: RawEndpointInfo[];
  selected_id?: string;
  success: boolean;
}

export interface SelectEndpointRequest {
  endpoint_id: string;
}

export interface EnhanceModeRequest {
  protocol?: Protocol;
  obfuscate?: boolean;
  obfuscateKey?: string;
  proxyMode?: ProxyMode;
}

export interface EnhanceModeResponse {
  success: boolean;
  message?: string;
  enhance_mode: EnhanceMode;
}

class WireGuardApiService {
  /**
   * Get current application mode (client or server)
   */
  async getMode(): Promise<AppMode> {
    const response = await apiClient.get<ModeResponse>('/api/mode');
    return response.mode;
  }

  /**
   * Get all available endpoints (from subscription or manual)
   */
  async getEndpoints(): Promise<EndpointsResponse> {
    return await apiClient.get<EndpointsResponse>('/api/endpoints');
  }

  /**
   * Select an endpoint
   */
  async selectEndpoint(endpointId: string): Promise<any> {
    return await apiClient.post<any>('/api/endpoints/select', {
      endpoint_id: endpointId,
    });
  }

  /**
   * Get current enhance mode settings
   */
  async getEnhanceMode(): Promise<EnhanceModeResponse> {
    return await apiClient.get<EnhanceModeResponse>('/api/settings/enhance-mode');
  }

  /**
   * Save enhance mode settings
   */
  async saveEnhanceMode(settings: EnhanceModeRequest): Promise<EnhanceModeResponse> {
    return await apiClient.post<EnhanceModeResponse>('/api/settings/enhance-mode', settings);
  }

  /**
   * Get WireGuard stats
   */
  async getStats(): Promise<any> {
    return await apiClient.get<any>('/api/getwgstats');
  }

  /**
   * Connect to VPN
   */
  async connect(endpointId: string): Promise<any> {
    return await apiClient.post<any>('/api/connect', { endpoint: endpointId });
  }

  /**
   * Disconnect from VPN
   */
  async disconnect(): Promise<any> {
    return await apiClient.post<any>('/api/disconnect', {});
  }

  /**
   * Get logs
   */
  async getLogs(): Promise<any> {
    return await apiClient.get<any>('/api/logs');
  }

  /**
   * Get subscription plans
   */
  async getSubscriptionPlans(): Promise<any> {
    return await apiClient.get<any>('/api/subscribe_plans');
  }

  /**
   * Get user info
   */
  async getUserInfo(): Promise<any> {
    return await apiClient.get<any>('/api/user_info');
  }
  
  /**
   * Add a custom endpoint
   */
  async addEndpoint(nodeLocation: string, nodeConfig: string): Promise<any> {
    return await apiClient.post<any>('/api/add_endpoint', {
      node_location: nodeLocation,
      node_config: nodeConfig,
    });
  }

  /**
   * Update an existing custom endpoint
   */
  async updateEndpoint(endpointId: string, nodeLocation: string, nodeConfig: string): Promise<any> {
    return await apiClient.post<any>('/api/endpoints/update', {
      endpoint_id: endpointId,
      node_location: nodeLocation,
      node_config: nodeConfig,
    });
  }

  /**
   * Delete a custom endpoint
   */
  async deleteEndpoint(endpointId: string): Promise<any> {
    return await apiClient.post<any>('/api/endpoints/delete', {
      endpoint_id: endpointId,
    });
  }

  /**
   * Enable gateway mode
   */
  async enableGateway(): Promise<any> {
    return await apiClient.post<any>('/api/gateway/on', {});
  }

  /**
   * Disable gateway mode
   */
  async disableGateway(): Promise<any> {
    return await apiClient.post<any>('/api/gateway/off', {});
  }
}

export const wireguardApi = new WireGuardApiService();
