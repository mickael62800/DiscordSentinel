import { onMounted, onUnmounted } from "vue";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useFetch } from "./useFetch";
import type { ServerStats } from "../types";

export function useDashboard() {
  const { data: stats, loading, error, refresh: fetchStats } = useFetch<ServerStats | null>(
    "get_dashboard_stats",
    null,
  );

  let unlisten: UnlistenFn | null = null;

  onMounted(async () => {
    unlisten = await listen<{ event: string }>("ws:event", (e) => {
      if (e.payload.event === "bot_status") {
        fetchStats();
      }
    });
  });

  onUnmounted(() => {
    if (unlisten) unlisten();
  });

  return { stats, loading, error, fetchStats };
}
