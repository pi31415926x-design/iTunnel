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

/**
 * 监听地址/端口只由 **Rust**（`src/main.rs` 等）从根目录 `.env` 读入并 `bind`；前端构建物不会读该文件。
 *
 * 默认不设置 `baseUrl`，用相对路径请求 `/api/...`，与当前页 **同源**（与 Actix 实际监听的 host/port 一致即可）。
 * 曾用写死的 `http://127.0.0.1:8181` 时，在绑定端口或浏览器 host（如 `localhost` vs `127.0.0.1`）与请求 URL 不一致会导致 `/api/mode` 等失败。
 *
 * 需要固定到某个绝对地址时，设置 `VITE_API_BASE`（无尾斜杠）。
 */
function resolveApiBase(): string {
  const v = import.meta.env.VITE_API_BASE;
  if (v !== undefined && v !== null && String(v).trim() !== '') {
    return String(v).replace(/\/$/, '');
  }
  return '';
}

class ApiClient {
  private baseUrl: string = resolveApiBase();
  private timeout: number = 10000; // 10 seconds

  async request<T>(
    path: string,
    method: HttpMethod = HttpMethod.GET,
    body?: any,
  ): Promise<T> {
    const pathPart = path.startsWith('/') ? path : `/${path}`;
    const url = this.baseUrl ? `${this.baseUrl}${pathPart}` : pathPart;
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
