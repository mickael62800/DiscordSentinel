import { useFetch } from "./useFetch";
import type { ServerStats } from "../types";

export function useDashboard() {
  const { data: stats, loading, error, refresh: fetchStats } = useFetch<ServerStats | null>(
    "get_dashboard_stats",
    null,
  );

  return { stats, loading, error, fetchStats };
}
