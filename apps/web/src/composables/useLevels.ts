import { ref, onMounted, watch } from "vue";
import type { LevelConfig, UserLevel, LevelReward, DiscordRole } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { levelsService } from "@/services/levelsService";
import { discordRolesService } from "@/services/discordRolesService";

export function useLevels() {
  const { success, error: showError } = useToast();
  const config = ref<LevelConfig | null>(null);
  const leaderboard = ref<UserLevel[]>([]);
  const rewards = ref<LevelReward[]>([]);
  const roles = ref<DiscordRole[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const { selectedGuildId } = useGuildSelector();

  async function fetchAll() {
    const guildId = selectedGuildId.value;
    if (!guildId) {
      config.value = null;
      leaderboard.value = [];
      rewards.value = [];
      roles.value = [];
      loading.value = false;
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      const [c, l, r, ro] = await Promise.all([
        levelsService.getConfig(guildId).catch(() => null),
        levelsService.getLeaderboard(guildId).catch(() => []),
        levelsService.getRewards(guildId).catch(() => []),
        discordRolesService.getAll(guildId).catch(() => []),
      ]);
      config.value = c;
      leaderboard.value = l ?? [];
      rewards.value = r ?? [];
      roles.value = ro ?? [];
    } catch (e) {
      error.value = "Impossible de charger les niveaux.";
      console.error("Erreur lors du chargement des niveaux :", e);
      showError("Impossible de charger les niveaux.");
    } finally {
      loading.value = false;
    }
  }

  async function setReward(level: number, roleId: string, source: string) {
    const guildId = selectedGuildId.value;
    if (!guildId) return;
    try {
      await levelsService.setReward(guildId, level, roleId, source);
      rewards.value = await levelsService.getRewards(guildId);
      success("Recompense de niveau enregistree avec succes.");
    } catch (e) {
      console.error("Erreur lors de l'enregistrement de la recompense :", e);
      showError("Erreur lors de l'enregistrement de la recompense.");
    }
  }

  async function deleteReward(level: number, source: string) {
    const guildId = selectedGuildId.value;
    if (!guildId) return;
    try {
      await levelsService.deleteReward(guildId, level, source);
      rewards.value = await levelsService.getRewards(guildId);
      success("Recompense de niveau supprimee avec succes.");
    } catch (e) {
      console.error("Erreur lors de la suppression de la recompense :", e);
      showError("Erreur lors de la suppression de la recompense.");
    }
  }

  onMounted(fetchAll);
  watch(selectedGuildId, fetchAll);

  return { config, leaderboard, rewards, roles, loading, error, fetchAll, setReward, deleteReward };
}
