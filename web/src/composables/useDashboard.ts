import { useFetch } from "./useFetch";
import { dashboardService } from "@/services/dashboardService";
import type { ServerStats } from "../types";

export function useDashboard() {
  const { data: stats, loading, error, refresh: fetchStats } = useFetch<ServerStats | null>(
    () => dashboardService.getStats(),
    null,
    "statistiques du tableau de bord",
  );

  return { stats, loading, error, fetchStats };
}
