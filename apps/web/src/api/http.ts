// Wrapper fetch qui embarque bearer API key + header X-Discord-Token, comme le fait
// ApiAdapter cote desktop. L'URL base est lue depuis la config stockee dans localStorage.

import { getApiConfig, getDiscordToken } from "./config";

export function apiBase(): string {
  return getApiConfig()?.api_url || import.meta.env.VITE_API_URL || "http://localhost:3000";
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

export async function httpGet<T>(path: string): Promise<T> {
  const r = await fetch(`${apiBase()}${path}`, { headers: headers() });
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

// Base URL AI (Python FastAPI), separee de l'API principale.
export function aiBase(): string {
  return import.meta.env.VITE_AI_API_URL || "http://localhost:8000";
}
