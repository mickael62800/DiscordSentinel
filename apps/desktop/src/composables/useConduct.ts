import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ConductConfig, UserConductPoints, ConductPointsLog } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

export function useConduct() {
  const { error: showError } = useToast();
  const config = ref<ConductConfig | null>(null);
  const leaderboard = ref<UserConductPoints[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const { selectedGuildId } = useGuildSelector();

  async function fetchConfig() {
    const guildId = selectedGuildId.value;
    if (!guildId) return;
    try {
      config.value = await invoke<ConductConfig>("get_conduct_config", { guildId });
    } catch (e) {
      error.value = "Impossible de charger la configuration de conduite.";
      console.error("Echec du chargement de la configuration de conduite :", e);
      showError("Impossible de charger la configuration de conduite.");
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
      error.value = "Impossible de charger le classement de conduite.";
      console.error("Echec du chargement du classement de conduite :", e);
      showError("Impossible de charger le classement de conduite.");
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

  return { config, leaderboard, loading, error, fetchConfig, fetchLeaderboard };
}

export function useConductDetail() {
  const { error: showError } = useToast();
  const points = ref<UserConductPoints | null>(null);
  const log = ref<ConductPointsLog[]>([]);
  const loading = ref(false);

  async function fetchDetail(guildId: string, userId: string) {
    loading.value = true;
    try {
      points.value = await invoke<UserConductPoints>("get_conduct_points", { guildId, userId });
      log.value = await invoke<ConductPointsLog[]>("get_conduct_log", { guildId, userId });
    } catch (e) {
      console.error("Echec du chargement du detail de conduite :", e);
      showError("Impossible de charger le detail de conduite.");
    } finally {
      loading.value = false;
    }
  }

  return { points, log, loading, fetchDetail };
}
