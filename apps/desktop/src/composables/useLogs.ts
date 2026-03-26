import { ref, computed } from "vue";
import { useFetch } from "./useFetch";
import type { LogEntry } from "../types";

export function useLogs() {
  const { data: logs, loading, refresh: fetchLogs } = useFetch<LogEntry[]>("get_logs", []);
  const filterLevel = ref("all");
  const filterBot = ref("all");

  const filteredLogs = computed(() => {
    return logs.value.filter((log) => {
      if (filterLevel.value !== "all" && log.level !== filterLevel.value) return false;
      if (filterBot.value !== "all" && log.bot !== filterBot.value) return false;
      return true;
    });
  });

  const bots = computed(() => Array.from(new Set(logs.value.map((l) => l.bot))));

  return { logs, filteredLogs, bots, loading, filterLevel, filterBot, fetchLogs };
}
