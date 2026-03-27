import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ConductConfig, UserConductPoints, ConductPointsLog } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useConduct() {
  const config = ref<ConductConfig | null>(null);
  const leaderboard = ref<UserConductPoints[]>([]);
  const loading = ref(true);
  const { selectedGuildId } = useGuildSelector();

  async function fetchConfig() {
    const guildId = selectedGuildId.value;
    if (!guildId) return;
    try {
      config.value = await invoke<ConductConfig>("get_conduct_config", { guildId });
    } catch (e) {
      console.error("Failed to fetch conduct config:", e);
    }
  }

  async function fetchLeaderboard() {
    const guildId = selectedGuildId.value;
    if (!guildId) {
      leaderboard.value = [];
      loading.value = false;
      return;
    }
    loading.value = true;
    try {
      leaderboard.value = await invoke<UserConductPoints[]>("get_conduct_leaderboard", { guildId });
    } catch (e) {
      console.error("Failed to fetch leaderboard:", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(() => {
    fetchConfig();
    fetchLeaderboard();
  });

  watch(selectedGuildId, () => {
    fetchConfig();
    fetchLeaderboard();
  });

  return { config, leaderboard, loading, fetchConfig, fetchLeaderboard };
}

export function useConductDetail() {
  const points = ref<UserConductPoints | null>(null);
  const log = ref<ConductPointsLog[]>([]);
  const loading = ref(false);

  async function fetchDetail(guildId: string, userId: string) {
    loading.value = true;
    try {
      points.value = await invoke<UserConductPoints>("get_conduct_points", { guildId, userId });
      log.value = await invoke<ConductPointsLog[]>("get_conduct_log", { guildId, userId });
    } catch (e) {
      console.error("Failed to fetch conduct detail:", e);
    } finally {
      loading.value = false;
    }
  }

  return { points, log, loading, fetchDetail };
}
