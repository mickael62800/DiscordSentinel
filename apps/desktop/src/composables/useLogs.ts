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

  // Liste statique de tous les bots + ceux présents dans les logs
  const knownBots = [
    "automod-bot",
    "moderation-bot",
    "security-bot",
    "stats-bot",
    "ticket-bot",
    "image-bot",
    "voice-bot",
    "audit-bot",
    "roles-bot",
  ];
  const bots = computed(() => {
    const fromLogs = logs.value.map((l) => l.bot);
    return Array.from(new Set([...knownBots, ...fromLogs])).sort();
  });

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
