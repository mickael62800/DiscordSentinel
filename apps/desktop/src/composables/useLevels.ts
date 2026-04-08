import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { LevelConfig, UserLevel, LevelReward, DiscordRole } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

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
        invoke<LevelConfig>("get_level_config", { guildId }).catch(() => null),
        invoke<UserLevel[]>("get_level_leaderboard", { guildId }).catch(() => []),
        invoke<LevelReward[]>("get_level_rewards", { guildId }).catch(() => []),
        invoke<DiscordRole[]>("get_discord_roles", { guildId }).catch(() => []),
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
      await invoke("set_level_reward", { guildId, level, roleId, source });
      // Rafraichir les rewards
      rewards.value = await invoke<LevelReward[]>("get_level_rewards", { guildId });
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
      await invoke("delete_level_reward", { guildId, level, source });
      rewards.value = await invoke<LevelReward[]>("get_level_rewards", { guildId });
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
