/**
 * Same-origin fetch for server Web UI: sends session cookies and sends the
 * browser to /login when the session expired (401).
 */
export function serverFetch(input: RequestInfo | URL, init?: RequestInit): Promise<Response> {
  return fetch(input, { ...init, credentials: 'include' }).then((res) => {
    if (
      res.status === 401 &&
      typeof window !== 'undefined' &&
      window.location.pathname !== '/login'
    ) {
      window.location.assign('/login');
    }
    return res;
  });
}
