import { httpGet, httpDelete } from "@/api/http";
import type { LogEntry } from "@/types";
import { q } from "./_query";

export const logsService = {
  getAll(guildId?: string | null): Promise<LogEntry[]> {
    return httpGet(`/api/logs${q({ guild_id: guildId ?? null })}`);
  },
  deleteByCategory(category: string): Promise<void> {
    return httpDelete(`/api/logs/${category}`);
  },
};
