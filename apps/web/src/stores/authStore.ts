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

    try {
      const currentUser = authService.getCurrentUser();
      if (currentUser) {
        user.value = currentUser;
        return;
      }

      const store = await getKv();
      const stored = await store.get<DiscordUser>(USER_KEY);
      if (stored) user.value = stored;
    } catch {
      // No session
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
