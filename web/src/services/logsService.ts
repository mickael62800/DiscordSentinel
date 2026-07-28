import { httpGet, httpDelete } from "@/api/http";
import type { LogEntry } from "@/types";
import { q } from "./_query";

export const logsService = {
  getAll(
    guildId?: string | null,
    category?: string | null,
    level?: string | null,
    limit?: number | null,
  ): Promise<LogEntry[]> {
    return httpGet(
      `/api/logs${q({
        guild_id: guildId ?? null,
        category: category ?? null,
        level: level && level !== "all" ? level : null,
        limit: limit ?? null,
      })}`,
    );
  },
  deleteByCategory(category: string): Promise<void> {
    return httpDelete(`/api/logs/${category}`);
  },
};
