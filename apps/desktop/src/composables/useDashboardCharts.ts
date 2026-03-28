import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { DailyActivity, TopUser } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useDashboardCharts() {
  const activity = ref<DailyActivity[]>([]);
  const topUsers = ref<TopUser[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const days = ref(30);
  const { guildIdFilter } = useGuildSelector();

  async function fetchAll() {
    loading.value = true;
    error.value = null;
    try {
      const [activityData, usersData] = await Promise.all([
        invoke<DailyActivity[]>("get_activity_trend", {
          guildId: guildIdFilter.value ?? null,
          days: days.value,
        }),
        guildIdFilter.value
          ? invoke<TopUser[]>("get_top_users", {
              guildId: guildIdFilter.value,
              limit: 10,
            })
          : Promise.resolve([]),
      ]);
      activity.value = activityData;
      topUsers.value = usersData;
    } catch (e) {
      error.value = String(e);
      activity.value = [];
      topUsers.value = [];
      console.error("Erreur chargement dashboard:", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchAll);
  watch([guildIdFilter, days], fetchAll);

  return { activity, topUsers, loading, error, days, fetchAll };
}
