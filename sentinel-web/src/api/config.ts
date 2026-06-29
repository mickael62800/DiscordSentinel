// Config persistante (API url + tokens bots) en localStorage pour la version web.
// Le client_id/secret Discord OAuth est gere cote backend, le front n'en
// voit jamais rien.

const K_API = "ds.api.config";
const K_DISCORD_USER = "ds.discord.user";

export interface ApiConfig { api_url: string; api_key: string }
export interface DiscordUser { id: string; username: string; avatar?: string | null; global_name?: string | null }

export function getApiConfig(): ApiConfig | null {
  const raw = localStorage.getItem(K_API);
  return raw ? JSON.parse(raw) : null;
}
export function setApiConfig(cfg: ApiConfig) {
  localStorage.setItem(K_API, JSON.stringify(cfg));
}

export function getDiscordUser(): DiscordUser | null {
  const raw = localStorage.getItem(K_DISCORD_USER);
  return raw ? JSON.parse(raw) : null;
}
export function setDiscordUser(u: DiscordUser | null) {
  if (u) localStorage.setItem(K_DISCORD_USER, JSON.stringify(u));
  else localStorage.removeItem(K_DISCORD_USER);
}

// Token Discord OAuth (renseigne apres callback OAuth) envoye en header X-Discord-Token.
//
// SECURITE : stocke en sessionStorage (et non localStorage) pour limiter
// l'exfiltration en cas de XSS persistant. sessionStorage est purge a la
// fermeture du tab/navigateur -> un attaquant doit voler le token "live"
// pendant que le tab est ouvert. Migration douce : on lit aussi l'ancienne
// valeur localStorage pour les sessions existantes, puis on la deplace.
const K_DISCORD_TOKEN = "ds.discord.token";

function migrateFromLocalStorage(): void {
  const legacy = localStorage.getItem(K_DISCORD_TOKEN);
  if (legacy && !sessionStorage.getItem(K_DISCORD_TOKEN)) {
    sessionStorage.setItem(K_DISCORD_TOKEN, legacy);
  }
  if (legacy) {
    localStorage.removeItem(K_DISCORD_TOKEN);
  }
}

export function getDiscordToken(): string {
  migrateFromLocalStorage();
  return sessionStorage.getItem(K_DISCORD_TOKEN) ?? "";
}
export function setDiscordToken(t: string) {
  sessionStorage.setItem(K_DISCORD_TOKEN, t);
  // Au cas ou un ancien token traine en localStorage, on le purge.
  localStorage.removeItem(K_DISCORD_TOKEN);
}
export function clearDiscordToken() {
  sessionStorage.removeItem(K_DISCORD_TOKEN);
  localStorage.removeItem(K_DISCORD_TOKEN);
}
