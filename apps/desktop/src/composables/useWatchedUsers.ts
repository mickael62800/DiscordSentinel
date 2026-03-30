import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { WatchedUser, UserDossier } from "../types";
import { useGuildFetch } from "./useGuildFetch";

export function useWatchedUsers() {
  const { data: users, loading, error, refresh: fetchUsers } = useGuildFetch<WatchedUser[]>(
    "get_watched_users",
    [],
  );

  const selectedUser = ref<WatchedUser | null>(null);
  const dossier = ref<UserDossier | null>(null);
  const dossierLoading = ref(false);

  const searchQuery = ref("");
  const riskFilter = ref("");

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
