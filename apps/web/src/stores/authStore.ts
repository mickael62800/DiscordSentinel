import { defineStore } from "pinia";
import { ref } from "vue";
import { authService } from "@/services/authService";
import { configService } from "@/services/configService";
import { Store as KvStore } from "@/api/store";
import type { DiscordUser } from "@/api/config";

const STORE_FILE = "auth.json";
const USER_KEY = "discord_user";

async function getKv() { return KvStore.load(STORE_FILE); }

export const useAuthStore = defineStore("auth", () => {
  const user = ref<DiscordUser | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const initialized = ref(false);
  const hasConfig = ref(false);

  async function checkSession() {
    if (initialized.value) return;
    initialized.value = true;

    // hasConfig = uniquement la config API (URL + cle). Le OAuth Discord
    // est desormais gere cote backend, le front n'a plus besoin de
    // client_id/client_secret localement.
    hasConfig.value = configService.getApiConfig() !== null;
    if (!hasConfig.value) return;

    // Restore le user depuis le storage (rapide).
    try {
      const currentUser = authService.getCurrentUser();
      if (currentUser) {
        user.value = currentUser;
      } else {
        const store = await getKv();
        const stored = await store.get<DiscordUser>(USER_KEY);
        if (stored) user.value = stored;
      }
    } catch {
      // Pas de session locale.
    }

    // Si on a un user en cache, valide que le token Discord est encore
    // accepte par l'API. Si l'endpoint refuse (401/403/503), on purge la
    // session pour forcer une re-authentification. Evite que le user reste
    // "connecte" cote front avec un token expire qui spam des 401.
    if (user.value) {
      try {
        const { httpGet } = await import("@/api/http");
        // /api/auth/check-access exige X-Discord-Token valide. Sur token
        // expire, l'API retourne 401 -> http.ts purge auto + redirige
        // vers /login. Sur 403, on purge manuellement aussi.
        await httpGet("/api/auth/check-access");
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        if (msg.includes("session expired") || msg.includes("403")) {
          // http.ts a deja redirige sur 401. Sur 403, on clear ici.
          if (!msg.includes("session expired")) {
            user.value = null;
            try {
              const store = await getKv();
              await store.delete(USER_KEY);
            } catch { /* ignore */ }
          }
        }
        // Autres erreurs (network) : on laisse passer, l'app re-essayera.
      }
    }
  }

  async function saveConfig(clientId: string, clientSecret: string) {
    configService.saveDiscordConfig(clientId, clientSecret);
    hasConfig.value = true;
  }

  async function clearConfig() {
    configService.clearDiscordConfig();
    hasConfig.value = false;
  }

  async function login() {
    loading.value = true;
    error.value = null;
    try {
      const loggedUser = await authService.discordLogin();
      user.value = loggedUser;
      const store = await getKv();
      await store.set(USER_KEY, loggedUser);
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function logout() {
    authService.logout();
    user.value = null;
    initialized.value = false;
    const store = await getKv();
    await store.delete(USER_KEY);
  }

  function avatarUrl(u: DiscordUser & { discriminator?: string }): string {
    if (u.avatar) {
      return `https://cdn.discordapp.com/avatars/${u.id}/${u.avatar}.png?size=64`;
    }
    const index = u.discriminator === "0"
      ? (BigInt(u.id) >> 22n) % 6n
      : Number(u.discriminator ?? 0) % 5;
    return `https://cdn.discordapp.com/embed/avatars/${index}.png`;
  }

  return {
    user, loading, error, initialized, hasConfig,
    checkSession, saveConfig, clearConfig, login, logout, avatarUrl,
  };
});
