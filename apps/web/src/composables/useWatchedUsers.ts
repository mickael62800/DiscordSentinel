import { ref } from "vue";
import type { WatchedUser, UserDossier } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { useToast } from "./useToast";
import { watchedUsersService } from "@/services/watchedUsersService";

export function useWatchedUsers() {
  const { data: users, loading, error, refresh: fetchUsers } = useGuildFetch<WatchedUser[]>(
    (guildId) => watchedUsersService.getAll(guildId),
    [],
    { label: "utilisateurs surveilles" },
  );

  const { error: showError } = useToast();
  const selectedUser = ref<WatchedUser | null>(null);
  const dossier = ref<UserDossier | null>(null);
  const dossierLoading = ref(false);

  const searchQuery = ref("");
  const riskFilter = ref("");

  async function fetchDossier(guildId: string, userId: string) {
    dossierLoading.value = true;
    try {
      dossier.value = await watchedUsersService.getDossier(guildId, userId);
    } catch (e) {
      console.error("Erreur lors du chargement du dossier :", e);
      showError("Erreur lors du chargement du dossier.");
    } finally {
      dossierLoading.value = false;
    }
  }

  function selectUser(user: WatchedUser | null) {
    selectedUser.value = user;
    if (user) fetchDossier(user.guild_id, user.user_id);
    else dossier.value = null;
  }

  return {
    users, loading, error, searchQuery, riskFilter,
    selectedUser, dossier, dossierLoading, fetchUsers, selectUser,
  };
}
