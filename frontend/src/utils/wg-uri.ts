/**
 * wg:// and wireguard:// carry the full [Interface]/[Peer] text as UTF-8, Base64URL-encoded (no padding).
 */

const WG_URI_RE = /^(?:wg|wireguard):\/\/(.+)$/i;

export function wgUriToConf(uri: string): string {
  const trimmed = uri.trim();
  const m = trimmed.match(WG_URI_RE);
  const payload = m?.[1];
  if (payload == null) {
    throw new Error('Not a wg:// or wireguard:// URI');
  }
  let b64 = payload.trim();
  try {
    if (b64.includes('%')) {
      b64 = decodeURIComponent(b64);
    }
  } catch {
    throw new Error('Invalid encoding in wg:// URI');
  }
  const padLen = (4 - (b64.length % 4)) % 4;
  const pad = padLen ? '='.repeat(padLen) : '';
  const std = (b64 + pad).replace(/-/g, '+').replace(/_/g, '/');
  let binary: string;
  try {
    binary = atob(std);
  } catch {
    throw new Error('Invalid Base64 payload in wg:// URI');
  }
  const bytes = Uint8Array.from(binary, (c) => c.charCodeAt(0));
  const conf = new TextDecoder('utf-8', { fatal: false }).decode(bytes).trim();
  if (!conf.includes('[Interface]') && !conf.includes('[Peer]')) {
    throw new Error('Decoded wg:// payload is not WireGuard configuration text');
  }
  return conf;
}

/** QR/text payload → conf string for the API (wg:// or raw .conf). */
export function parseWireguardImportPayload(payload: string): string {
  const t = payload.trim();
  if (!t) {
    throw new Error('Empty payload');
  }
  if (WG_URI_RE.test(t)) {
    return wgUriToConf(t);
  }
  if (t.includes('[Interface]') || t.includes('[Peer]')) {
    return t;
  }
  throw new Error('Expected wg:// URI or WireGuard [Interface]/[Peer] text');
}

export function confToWgUri(conf: string): string {
  const bytes = new TextEncoder().encode(conf.trim());
  let bin = '';
  bytes.forEach((b) => {
    bin += String.fromCharCode(b);
  });
  const b64 = btoa(bin).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  return `wg://${b64}`;
}
