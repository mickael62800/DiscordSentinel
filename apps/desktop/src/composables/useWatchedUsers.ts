import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { WatchedUser, UserDossier } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useWatchedUsers() {
  const users = ref<WatchedUser[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const { guildIdFilter } = useGuildSelector();

  const selectedUser = ref<WatchedUser | null>(null);
  const dossier = ref<UserDossier | null>(null);
  const dossierLoading = ref(false);

  const searchQuery = ref("");
  const riskFilter = ref("");

  async function fetchUsers() {
    loading.value = true;
    error.value = null;
    try {
      users.value = await invoke<WatchedUser[]>("get_watched_users", {
        guildId: guildIdFilter.value ?? null,
      });
    } catch (e) {
      error.value = String(e);
      console.error("Erreur chargement utilisateurs surveilles:", e);
    } finally {
      loading.value = false;
    }
  }

  async function fetchDossier(guildId: string, userId: string) {
    dossierLoading.value = true;
    try {
      dossier.value = await invoke<UserDossier>("get_user_dossier", {
        guildId,
        userId,
      });
    } catch (e) {
      console.error("Erreur chargement dossier:", e);
    } finally {
      dossierLoading.value = false;
    }
  }

  function selectUser(user: WatchedUser | null) {
    selectedUser.value = user;
    if (user) {
      fetchDossier(user.guild_id, user.user_id);
    } else {
      dossier.value = null;
    }
  }

  onMounted(fetchUsers);
  watch(guildIdFilter, fetchUsers);

  return {
    users,
    loading,
    error,
    searchQuery,
    riskFilter,
    selectedUser,
    dossier,
    dossierLoading,
    fetchUsers,
    selectUser,
  };
}
