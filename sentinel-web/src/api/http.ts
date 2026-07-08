// Wrapper fetch qui embarque bearer API key + header X-Discord-Token, comme le fait
// ApiAdapter cote desktop. L'URL base est lue depuis la config stockee dans localStorage.
//
// Persistance "rester connecte" : un cookie httpOnly `ds_session` (pose au
// callback OAuth) permet de re-emettre un token d'acces via POST /auth/refresh
// sans re-validation Discord interactive. Sur 401, on tente un refresh
// transparent puis on rejoue la requete une fois avant de rediriger sur /login.

import { getApiConfig, getDiscordToken, clearDiscordToken, setDiscordToken, setDiscordUser } from "./config";

export function apiBase(): string {
  // Priorite : config localStorage utilisateur > VITE_API_URL au build > defaut.
  // En prod, defaut "" -> URLs relatives -> passent par le proxy nginx (origin courant).
  // En dev, defaut http://localhost:3000 -> hit l'API directement.
  // SECURITE : la valeur vient de localStorage (modifiable par n'importe quel
  // code de la page). On n'accepte que http(s) — jamais javascript:, data:,
  // etc. — pour eviter qu'une config empoisonnee detourne les requetes
  // (et les tokens qu'elles embarquent) vers un schema/URL arbitraire.
  const cfg = getApiConfig()?.api_url;
  if (cfg) {
    try {
      const u = new URL(cfg);
      if (u.protocol === "https:" || u.protocol === "http:") return cfg;
    } catch { /* URL malformee : ignore, fallback env/defaut */ }
  }
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

// ── Refresh de session (cookie httpOnly) ──
let refreshInFlight: Promise<boolean> | null = null;

/**
 * Tente de ré-émettre un token d'accès Discord via le cookie de session.
 * Met à jour le token + l'identité en cache. Dédupe les appels concurrents.
 * Retourne true si un nouveau token a été obtenu.
 */
export async function tryRefreshSession(): Promise<boolean> {
  if (refreshInFlight) return refreshInFlight;
  refreshInFlight = (async () => {
    try {
      const r = await fetch(`${apiBase()}/auth/refresh`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
      });
      if (!r.ok) return false;
      const data = await r.json().catch(() => null);
      if (!data?.token) return false;
      setDiscordToken(data.token);
      setDiscordUser({
        id: data.id,
        username: data.username,
        global_name: data.global_name ?? null,
        avatar: data.avatar ?? null,
      });
      return true;
    } catch {
      return false;
    }
  })();
  const ok = await refreshInFlight;
  refreshInFlight = null;
  return ok;
}

/** Supprime la session serveur + le cookie (logout propre). */
export async function logoutSession(): Promise<void> {
  try {
    await fetch(`${apiBase()}/auth/logout`, { method: "POST", credentials: "include" });
  } catch { /* best-effort */ }
}

async function handle<T>(resp: Response): Promise<T> {
  if (!resp.ok) {
    if (resp.status === 401) {
      const path = window.location.pathname;
      // Cas particulier : si on est sur /auth/callback, NE PAS purger le
      // token. Une requete zombie partie avant l'OAuth callback pourrait
      // recevoir son 401 APRES que AuthCallbackPage ait stocke le nouveau
      // token, et le clear effacerait la session toute fraiche -> boucle
      // /login?expired=1 garantie. On laisse AuthCallbackPage gerer son
      // cycle de vie en paix.
      if (path.startsWith("/auth/")) {
        throw new Error("Unauthorized: session expired");
      }
      // Token invalide/expire ET refresh impossible : on purge la session
      // locale et on redirige vers /login pour forcer une re-authentification.
      try {
        clearDiscordToken();
        localStorage.removeItem("ds.discord.user");
      } catch { /* storage quota / cookies disabled : ignore */ }
      if (path !== "/login") {
        window.location.href = "/login?expired=1";
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

/**
 * Coeur des appels : pose toujours `credentials:'include'` (cookie de session)
 * et, sur 401 hors flux /auth, tente un refresh transparent + rejoue 1 fois.
 */
async function request<T>(
  path: string,
  method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE",
  body?: unknown,
): Promise<T> {
  const url = `${apiBase()}${path}`;
  const isGet = method === "GET";
  const build = (): RequestInit => ({
    method,
    headers: headers(),
    credentials: "include",
    body: body === undefined ? undefined : JSON.stringify(body),
  });

  let r = isGet ? await fetchWithRetry503(url, build()) : await fetch(url, build());

  // 401 : tentative de refresh silencieux (cookie) puis rejoue une fois.
  if (r.status === 401 && !path.startsWith("/auth/")) {
    const refreshed = await tryRefreshSession();
    if (refreshed) {
      r = isGet ? await fetchWithRetry503(url, build()) : await fetch(url, build());
    }
  }
  return handle<T>(r);
}

export async function httpGet<T>(path: string): Promise<T> {
  return request<T>(path, "GET");
}
export async function httpPost<T>(path: string, body?: unknown): Promise<T> {
  return request<T>(path, "POST", body);
}
export async function httpPut<T>(path: string, body?: unknown): Promise<T> {
  return request<T>(path, "PUT", body);
}
export async function httpPatch<T>(path: string, body?: unknown): Promise<T> {
  return request<T>(path, "PATCH", body);
}
export async function httpDelete<T>(path: string, body?: unknown): Promise<T> {
  return request<T>(path, "DELETE", body);
}
