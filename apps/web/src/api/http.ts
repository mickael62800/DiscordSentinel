// Wrapper fetch qui embarque bearer API key + header X-Discord-Token, comme le fait
// ApiAdapter cote desktop. L'URL base est lue depuis la config stockee dans localStorage.

import { getApiConfig, getDiscordToken } from "./config";

export function apiBase(): string {
  // Priorite : config localStorage utilisateur > VITE_API_URL au build > defaut.
  // En prod, defaut "" -> URLs relatives -> passent par le proxy nginx (origin courant).
  // En dev, defaut http://localhost:3000 -> hit l'API directement.
  const cfg = getApiConfig()?.api_url;
  if (cfg) return cfg;
  const env = import.meta.env.VITE_API_URL;
  if (env) return env;
  return import.meta.env.PROD ? "" : "http://localhost:3000";
}

function headers(extra?: Record<string, string>): Record<string, string> {
  const h: Record<string, string> = { "Content-Type": "application/json", ...extra };
  const cfg = getApiConfig();
  if (cfg?.api_key) h["Authorization"] = `Bearer ${cfg.api_key}`;
  const tok = getDiscordToken();
  if (tok) h["X-Discord-Token"] = tok;
  return h;
}

async function handle<T>(resp: Response): Promise<T> {
  if (!resp.ok) {
    if (resp.status === 401) throw new Error("Unauthorized: invalid API key");
    const body = await resp.text().catch(() => "");
    throw new Error(`API error ${resp.status}: ${body}`);
  }
  const txt = await resp.text();
  if (!txt) return undefined as unknown as T;
  try { return JSON.parse(txt) as T; } catch { return txt as unknown as T; }
}

/**
 * Retry exponentiel sur 503 (rate limit Discord, brefs incidents reseau).
 * Uniquement sur GET (idempotents). 3 tentatives max : 0ms / 500ms / 1500ms.
 * Evite les pages blanches quand le 503 dure < 2s (cas typique du middleware
 * guild_auth qui rebound apres un cache miss + Discord 429).
 */
async function fetchWithRetry503(url: string, init?: RequestInit): Promise<Response> {
  const delays = [0, 500, 1500];
  let last: Response | null = null;
  for (const d of delays) {
    if (d > 0) await new Promise((r) => setTimeout(r, d));
    last = await fetch(url, init);
    if (last.status !== 503) return last;
  }
  return last as Response;
}

export async function httpGet<T>(path: string): Promise<T> {
  const r = await fetchWithRetry503(`${apiBase()}${path}`, { headers: headers() });
  return handle<T>(r);
}
export async function httpPost<T>(path: string, body?: unknown): Promise<T> {
  const r = await fetch(`${apiBase()}${path}`, { method: "POST", headers: headers(), body: body === undefined ? undefined : JSON.stringify(body) });
  return handle<T>(r);
}
export async function httpPut<T>(path: string, body?: unknown): Promise<T> {
  const r = await fetch(`${apiBase()}${path}`, { method: "PUT", headers: headers(), body: body === undefined ? undefined : JSON.stringify(body) });
  return handle<T>(r);
}
export async function httpPatch<T>(path: string, body?: unknown): Promise<T> {
  const r = await fetch(`${apiBase()}${path}`, { method: "PATCH", headers: headers(), body: body === undefined ? undefined : JSON.stringify(body) });
  return handle<T>(r);
}
export async function httpDelete<T>(path: string, body?: unknown): Promise<T> {
  const r = await fetch(`${apiBase()}${path}`, { method: "DELETE", headers: headers(), body: body === undefined ? undefined : JSON.stringify(body) });
  return handle<T>(r);
}
