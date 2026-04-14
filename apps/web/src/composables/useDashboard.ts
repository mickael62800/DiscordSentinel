import { useFetch } from "./useFetch";
import { useRealtimeRefresh } from "./useRealtimeRefresh";
import { dashboardService } from "@/services/dashboardService";
import type { ServerStats } from "../types";

export function useDashboard() {
  const { data: stats, loading, error, refresh: fetchStats } = useFetch<ServerStats | null>(
    () => dashboardService.getStats(),
    null,
    "statistiques du tableau de bord",
  );

  useRealtimeRefresh(
    ["bot_heartbeat", "stats_messages_recorded", "stats_voice_recorded", "infraction_new", "moderation_action"],
    fetchStats,
    { debounceMs: 2000 },
  );

  return { stats, loading, error, fetchStats };
}
