import { ref, onMounted, watch, type Ref } from "vue";
import type { DailyActivity, TopUser } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { dashboardChartsService } from "@/services/dashboardChartsService";

/**
 * Si `externalDays` est fourni, la composable l'utilise comme source
 * unique (utile quand plusieurs sections partagent un sélecteur de
 * période global). Sinon, elle maintient son propre `days`.
 */
export function useDashboardCharts(externalDays?: Ref<number>) {
  const activity = ref<DailyActivity[]>([]);
  const topUsers = ref<TopUser[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const days = externalDays ?? ref(30);
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

  return { activity, topUsers, loading, error, days, fetchAll };
}
