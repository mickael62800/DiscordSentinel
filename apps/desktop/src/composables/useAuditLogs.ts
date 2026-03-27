import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { AuditLog } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useAuditLogs() {
  const logs = ref<AuditLog[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const { guildIdFilter } = useGuildSelector();

  const filterEventType = ref("");
  const searchQuery = ref("");

  const filteredLogs = computed(() => {
    let list = logs.value;
    if (filterEventType.value) {
      list = list.filter((l) => l.event_type === filterEventType.value);
    }
    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase();
      list = list.filter(
        (l) =>
          (l.actor_name?.toLowerCase().includes(q) ?? false) ||
          (l.target_name?.toLowerCase().includes(q) ?? false) ||
          (l.channel_name?.toLowerCase().includes(q) ?? false) ||
          l.event_type.toLowerCase().includes(q),
      );
    }
    return list;
  });

  const eventTypes = computed(() => {
    const types = new Set(logs.value.map((l) => l.event_type));
    return Array.from(types).sort();
  });

  async function fetchLogs() {
    loading.value = true;
    error.value = null;
    try {
      logs.value = await invoke<AuditLog[]>("get_audit_logs", {
        guildId: guildIdFilter.value ?? null,
        eventType: null,
        limit: 500,
      });
    } catch (e) {
      error.value = String(e);
      console.error("Erreur chargement audit logs:", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchLogs);
  watch(guildIdFilter, fetchLogs);

  return { logs, filteredLogs, eventTypes, loading, error, filterEventType, searchQuery, fetchLogs };
}
