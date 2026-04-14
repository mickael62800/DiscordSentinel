import { ref, onMounted, watch } from "vue";
import type { DailyActivity, TopUser } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useRealtimeRefresh } from "./useRealtimeRefresh";
import { dashboardChartsService } from "@/services/dashboardChartsService";

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
        dashboardChartsService.getActivityTrend(guildIdFilter.value ?? null, days.value),
        guildIdFilter.value
          ? dashboardChartsService.getTopUsers(guildIdFilter.value, 10)
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

  useRealtimeRefresh(
    ["stats_messages_recorded", "stats_voice_recorded", "infraction_new", "moderation_action"],
    fetchAll,
    { debounceMs: 5000 },
  );

  return { activity, topUsers, loading, error, days, fetchAll };
}
