import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { load } from "@tauri-apps/plugin-store";
import type { DiscordUser } from "../types";

const user = ref<DiscordUser | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);
const initialized = ref(false);
const hasConfig = ref(false);

const STORE_FILE = "auth.json";
const USER_KEY = "discord_user";

async function getStore() {
  return await load(STORE_FILE, { autoSave: true, defaults: {} });
}

export function useAuth() {
  async function checkSession() {
    if (initialized.value) return;
    initialized.value = true;

    // Check if Discord config exists in LMDB
    hasConfig.value = await invoke<boolean>("has_discord_config");

    if (!hasConfig.value) return;

    try {
      // Check Rust in-memory session
      const currentUser = await invoke<DiscordUser | null>("get_current_user");
      if (currentUser) {
        user.value = currentUser;
        return;
      }

      // Check persisted store
      const store = await getStore();
      const stored = await store.get<DiscordUser>(USER_KEY);
      if (stored) {
        user.value = stored;
      }
    } catch {
      // No session
    }
  }

  async function saveConfig(clientId: string, clientSecret: string) {
    await invoke("save_discord_config", { clientId, clientSecret });
    hasConfig.value = true;
  }

  async function clearConfig() {
    await invoke("clear_discord_config");
    hasConfig.value = false;
  }

  async function login() {
    loading.value = true;
    error.value = null;
    try {
      const loggedUser = await invoke<DiscordUser>("discord_login");
      user.value = loggedUser;

      // Persist to store
      const store = await getStore();
      await store.set(USER_KEY, loggedUser);
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  async function logout() {
    await invoke("logout");
    user.value = null;
    initialized.value = false;

    const store = await getStore();
    await store.delete(USER_KEY);
  }

  function avatarUrl(u: DiscordUser): string {
    if (u.avatar) {
      return `https://cdn.discordapp.com/avatars/${u.id}/${u.avatar}.png?size=64`;
    }
    const index = u.discriminator === "0"
      ? (BigInt(u.id) >> 22n) % 6n
      : Number(u.discriminator) % 5;
    return `https://cdn.discordapp.com/embed/avatars/${index}.png`;
  }

  return {
    user, loading, error, initialized, hasConfig,
    checkSession, saveConfig, clearConfig, login, logout, avatarUrl,
  };
}
