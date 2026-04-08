import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { LogEntry } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

export function useLogs(categoryFilter?: string) {
  const { error: showError } = useToast();
  const logs = ref<LogEntry[]>([]);
  const loading = ref(true);
  const filterLevel = ref("all");
  const filterBot = ref("all");
  const dateFrom = ref("");
  const dateTo = ref("");
  const search = ref("");
  const { guildIdFilter } = useGuildSelector();

  // Les logs discord sont filtres par guild, les logs bot/worker/api sont globaux
  const isGuildScoped = !categoryFilter || categoryFilter === "discord";

  const categoryLogs = computed(() => {
    if (!categoryFilter) return logs.value;
    return logs.value.filter((l) => l.category === categoryFilter);
  });

  const filteredLogs = computed(() => {
    return categoryLogs.value.filter((log) => {
      if (search.value) {
        const q = search.value.toLowerCase();
        const match = [log.message, log.bot, log.level, log.server, log.timestamp]
          .some((field) => field?.toLowerCase().includes(q));
        if (!match) return false;
      }
      if (filterLevel.value !== "all" && log.level !== filterLevel.value) return false;
      if (filterBot.value !== "all" && log.bot !== filterBot.value) return false;
      if (dateFrom.value) {
        const from = new Date(dateFrom.value).getTime();
        const logDate = new Date(log.timestamp).getTime();
        if (logDate < from) return false;
      }
      if (dateTo.value) {
        const to = new Date(dateTo.value + "T23:59:59").getTime();
        const logDate = new Date(log.timestamp).getTime();
        if (logDate > to) return false;
      }
      return true;
    });
  });

  const sources = computed(() => {
    const fromLogs = categoryLogs.value.map((l) => l.bot).filter(Boolean);
    return Array.from(new Set(fromLogs)).sort();
  });

  async function fetchLogs() {
    loading.value = true;
    try {
      const guildId = isGuildScoped ? (guildIdFilter.value ?? null) : null;
      logs.value = await invoke<LogEntry[]>("get_logs", { guildId });
    } catch (e) {
      console.error("Erreur lors du chargement des journaux :", e);
      showError("Erreur lors du chargement des journaux.");
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchLogs);
  if (isGuildScoped) {
    watch(guildIdFilter, fetchLogs);
  }

  async function clearLogs() {
    if (!categoryFilter || categoryFilter === "discord") return;
    try {
      await invoke("delete_logs_by_category", { category: categoryFilter });
      await fetchLogs();
    } catch (e) {
      console.error("Erreur lors de la suppression des logs :", e);
      showError("Erreur lors de la suppression des logs.");
    }
  }

  return { logs, categoryLogs, filteredLogs, sources, loading, filterLevel, filterBot, dateFrom, dateTo, search, fetchLogs, clearLogs };
}
