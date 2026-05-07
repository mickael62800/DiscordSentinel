import { defineStore } from "pinia";
import { ref } from "vue";
import { authService } from "@/services/authService";
import { Store as KvStore } from "@/api/store";
import { getDiscordToken } from "@/api/config";
import type { DiscordUser } from "@/api/config";

const STORE_FILE = "auth.json";
const USER_KEY = "discord_user";

async function getKv() { return KvStore.load(STORE_FILE); }

export const useAuthStore = defineStore("auth", () => {
  const user = ref<DiscordUser | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const initialized = ref(false);

  async function checkSession() {
    if (initialized.value) return;
    initialized.value = true;

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

    // Si on a un user en cache MAIS plus de token Discord en sessionStorage
    // (ex: tab ferme + rouvert, ou utilisateur revient de l'OAuth callback
    // avant que AuthCallbackPage ait pu stocker le nouveau token), on purge
    // le user obsolete sans pinguer l'API : la requete serait garantie 401
    // et redirigerait sur /login?expired=1, cassant notamment le retour
    // OAuth (/auth/callback) ou le token n'est pas encore en place.
    if (user.value && !getDiscordToken()) {
      user.value = null;
      try {
        const store = await getKv();
        await store.delete(USER_KEY);
      } catch { /* ignore */ }
      return;
    }

    // Si on a un user en cache, valide que le token Discord est encore
    // accepte par l'API. Sur 401 (token expire) -> http.ts purge + redirige.
    // Sur 403 (whitelist_middleware refuse : user pas/plus dans api_user_guilds)
    // on clear localement et on redirige sur /login?error=not_invited pour
    // proposer un code d'invitation.
    if (user.value) {
      try {
        const { httpGet } = await import("@/api/http");
        await httpGet("/api/auth/check-access");
      } catch (e) {
        const msg = e instanceof Error ? e.message : String(e);
        if (msg.includes("403")) {
          user.value = null;
          try {
            const store = await getKv();
            await store.delete(USER_KEY);
          } catch { /* ignore */ }
          // Redirect manuel vers login avec message explicite.
          if (window.location.pathname !== "/login") {
            window.location.href = "/login?error=not_invited";
          }
        }
        // 401 / network : http.ts gere ou on laisse passer.
      }
    }
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
    user, loading, error, initialized,
    checkSession, login, logout, avatarUrl,
  };
});
