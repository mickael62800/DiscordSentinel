import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { DailyActivity } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useDashboardCharts() {
  const activity = ref<DailyActivity[]>([]);
  const loading = ref(true);
  const days = ref(30);
  const { guildIdFilter } = useGuildSelector();

  async function fetchActivity() {
    loading.value = true;
    try {
      activity.value = await invoke<DailyActivity[]>("get_activity_trend", {
        guildId: guildIdFilter.value ?? null,
        days: days.value,
      });
    } catch (e) {
      console.error("Erreur chargement activite:", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchActivity);
  watch([guildIdFilter, days], fetchActivity);

  return { activity, loading, days, fetchActivity };
}
