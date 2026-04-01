import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { LevelConfig, UserLevel, LevelReward } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useLevels() {
  const config = ref<LevelConfig | null>(null);
  const leaderboard = ref<UserLevel[]>([]);
  const rewards = ref<LevelReward[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const { selectedGuildId } = useGuildSelector();

  async function fetchAll() {
    const guildId = selectedGuildId.value;
    if (!guildId) {
      config.value = null;
      leaderboard.value = [];
      rewards.value = [];
      loading.value = false;
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      const [c, l, r] = await Promise.all([
        invoke<LevelConfig>("get_level_config", { guildId }).catch(() => null),
        invoke<UserLevel[]>("get_level_leaderboard", { guildId }),
        invoke<LevelReward[]>("get_level_rewards", { guildId }),
      ]);
      config.value = c;
      leaderboard.value = l;
      rewards.value = r;
    } catch (e) {
      error.value = "Impossible de charger les niveaux.";
      console.error("Erreur chargement niveaux:", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchAll);
  watch(selectedGuildId, fetchAll);

  return { config, leaderboard, rewards, loading, error, fetchAll };
}
