// Wrapper fetch qui embarque bearer API key + header X-Discord-Token, comme le fait
// ApiAdapter cote desktop. L'URL base est lue depuis la config stockee dans localStorage.

import { getApiConfig, getDiscordToken, clearDiscordToken } from "./config";

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
  console.log("[http] headers built, hasToken:", !!tok, "hasApiKey:", !!cfg?.api_key);
  return h;
}

async function handle<T>(resp: Response): Promise<T> {
  if (!resp.ok) {
    if (resp.status === 401) {
      console.error("[http] 401 sur", resp.url, "- path actuel:", window.location.pathname, "- token present:", !!getDiscordToken());
      // Token invalide/expire : on purge la session locale et on redirige
      // vers /login pour forcer une re-authentification. Evite la boucle
      // infinie ou le user reste "logge" cote front mais 401 cote API.
      try {
        clearDiscordToken();
        localStorage.removeItem("ds.discord.user");
      } catch { /* storage quota / cookies disabled : ignore */ }
      const path = window.location.pathname;
      // Skip si deja sur login/setup/auth-callback (eviter boucle de redir).
      if (path !== "/login" && path !== "/setup" && !path.startsWith("/auth/")) {
        console.error("[http] redirect HARD vers /login?expired=1");
        // Soft redirect via window.location pour reset l'app entiere
        // (Pinia stores, composables singletons, etc.).
        window.location.href = "/login?expired=1";
      } else {
        console.warn("[http] 401 sur path public, pas de redirect");
      }
      throw new Error("Unauthorized: session expired");
    }
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
