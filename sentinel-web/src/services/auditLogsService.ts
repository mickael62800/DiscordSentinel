import { httpDelete, httpGet } from "@/api/http";
import type { AuditLog } from "@/types";
import { q } from "./_query";

export const auditLogsService = {
  getAll(guildId?: string | null, eventType?: string | null, limit?: number | null): Promise<AuditLog[]> {
    return httpGet(`/api/audit-logs${q({ guild_id: guildId ?? null, event_type: eventType ?? null, limit: limit ?? null })}`);
  },
  purge(guildId: string): Promise<{ deleted: number }> {
    return httpDelete(`/api/audit-logs/${guildId}`);
  },
};
