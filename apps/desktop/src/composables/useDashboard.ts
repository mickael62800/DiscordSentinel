import { useFetch } from "./useFetch";
import { useRealtimeRefresh } from "./useRealtimeRefresh";
import type { ServerStats } from "../types";

export function useDashboard() {
  const { data: stats, loading, error, refresh: fetchStats } = useFetch<ServerStats | null>(
    "get_dashboard_stats",
    null,
  );

  // Refresh automatique quand un bot heartbeat arrive ou quand des stats changent
  useRealtimeRefresh(
    ["bot_heartbeat", "stats_messages_recorded", "stats_voice_recorded", "infraction_new", "moderation_action"],
    fetchStats,
    { debounceMs: 2000 }, // Debounce 2s pour eviter les rafales
  );

  return { stats, loading, error, fetchStats };
}
