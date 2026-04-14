import { httpGet } from "@/api/http";
import type { FullAnalytics } from "@/types";
import { q } from "./_query";

export const analyticsService = {
  getFull(guildId?: string | null, days = 30): Promise<FullAnalytics> {
    return httpGet(`/api/analytics${q({ guild_id: guildId ?? null, days })}`);
  },
};
