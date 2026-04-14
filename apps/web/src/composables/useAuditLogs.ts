import { ref, computed } from "vue";
import type { AuditLog } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { auditLogsService } from "@/services/auditLogsService";

export function useAuditLogs() {
  const { data: logs, loading, error, refresh: fetchLogs } = useGuildFetch<AuditLog[]>(
    (guildId) => auditLogsService.getAll(guildId, null, 500),
    [],
    { label: "journaux d'audit" },
  );

  const filterEventType = ref("");
  const searchQuery = ref("");

  const filteredLogs = computed(() => {
    let list = logs.value;
    if (filterEventType.value) list = list.filter((l) => l.event_type === filterEventType.value);
    if (searchQuery.value) {
      const q = searchQuery.value.toLowerCase();
      list = list.filter((l) =>
        [l.actor_name, l.actor_id, l.target_name, l.target_id, l.channel_name, l.channel_id, l.event_type, l.created_at, JSON.stringify(l.details)]
          .some((field) => field?.toLowerCase().includes(q)),
      );
    }
    return list;
  });

  const eventTypes = computed(() => {
    const types = new Set(logs.value.map((l) => l.event_type));
    return Array.from(types).sort();
  });

  return { logs, filteredLogs, eventTypes, loading, error, filterEventType, searchQuery, fetchLogs };
}
