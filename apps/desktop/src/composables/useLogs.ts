import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { LogEntry } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useLogs() {
  const logs = ref<LogEntry[]>([]);
  const loading = ref(true);
  const filterLevel = ref("all");
  const filterBot = ref("all");
  const { guildIdFilter } = useGuildSelector();

  const filteredLogs = computed(() => {
    return logs.value.filter((log) => {
      if (filterLevel.value !== "all" && log.level !== filterLevel.value) return false;
      if (filterBot.value !== "all" && log.bot !== filterBot.value) return false;
      return true;
    });
  });

  const bots = computed(() => Array.from(new Set(logs.value.map((l) => l.bot))));

  async function fetchLogs() {
    loading.value = true;
    try {
      logs.value = await invoke<LogEntry[]>("get_logs", { guildId: guildIdFilter.value ?? null });
    } catch (e) {
      console.error("Erreur chargement journaux:", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchLogs);
  watch(guildIdFilter, fetchLogs);

  return { logs, filteredLogs, bots, loading, filterLevel, filterBot, fetchLogs };
}
