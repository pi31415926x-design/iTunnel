/**
 * Base API Service Layer
 * Handles all HTTP communication with the backend
 * Provides unified error handling, logging, and response parsing
 */

export interface ApiResponse<T> {
  success: boolean;
  data?: T;
  message?: string;
  error?: string;
}

export enum HttpMethod {
  GET = 'GET',
  POST = 'POST',
  PUT = 'PUT',
  DELETE = 'DELETE',
  PATCH = 'PATCH',
}

class ApiClient {
  private baseUrl: string = 'http://127.0.0.1:8181';
  private timeout: number = 10000; // 10 seconds

  async request<T>(
    path: string,
    method: HttpMethod = HttpMethod.GET,
    body?: any,
  ): Promise<T> {
    const url = `${this.baseUrl}${path}`;
    const options: RequestInit = {
      method,
      headers: {
        'Content-Type': 'application/json',
      },
    };

    if (body && (method === HttpMethod.POST || method === HttpMethod.PUT || method === HttpMethod.PATCH)) {
      options.body = JSON.stringify(body);
    }

    try {
      const controller = new AbortController();
      const timeoutId = setTimeout(() => controller.abort(), this.timeout);

      const response = await fetch(url, {
        ...options,
        signal: controller.signal,
      });

      clearTimeout(timeoutId);

      if (!response.ok) {
        throw new Error(`HTTP ${response.status}: ${response.statusText}`);
      }

      const data = await response.json();
      return data as T;
    } catch (error) {
      console.error(`API Error [${method} ${path}]:`, error);
      throw error;
    }
  }

  get<T>(path: string): Promise<T> {
    return this.request<T>(path, HttpMethod.GET);
  }

  post<T>(path: string, body: any): Promise<T> {
    return this.request<T>(path, HttpMethod.POST, body);
  }

  put<T>(path: string, body: any): Promise<T> {
    return this.request<T>(path, HttpMethod.PUT, body);
  }

  delete<T>(path: string): Promise<T> {
    return this.request<T>(path, HttpMethod.DELETE);
  }
}

export const apiClient = new ApiClient();
