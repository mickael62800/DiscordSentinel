import { ref, onMounted, watch } from "vue";
import type { UserLevel, DiscordRole } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { levelsService } from "@/services/levelsService";
import { discordRolesService } from "@/services/discordRolesService";

export function useLevels() {
  const { error: showError } = useToast();
  const leaderboard = ref<UserLevel[]>([]);
  const roles = ref<DiscordRole[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const { selectedGuildId } = useGuildSelector();

  async function fetchAll() {
    const guildId = selectedGuildId.value;
    if (!guildId) {
      leaderboard.value = [];
      roles.value = [];
      loading.value = false;
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      const [l, ro] = await Promise.all([
        levelsService.getLeaderboard(guildId).catch(() => []),
        discordRolesService.getAll(guildId).catch(() => []),
      ]);
      leaderboard.value = l ?? [];
      roles.value = ro ?? [];
    } catch (e) {
      error.value = "Impossible de charger les niveaux.";
      console.error("Erreur lors du chargement des niveaux :", e);
      showError("Impossible de charger les niveaux.");
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchAll);
  watch(selectedGuildId, fetchAll);

  return { leaderboard, roles, loading, error, fetchAll };
}
