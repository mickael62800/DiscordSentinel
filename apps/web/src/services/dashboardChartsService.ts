import { httpGet } from "@/api/http";
import type { DailyActivity, TopUser } from "@/types";
import { q } from "./_query";

export const dashboardChartsService = {
  getActivityTrend(guildId?: string | null, days = 30): Promise<DailyActivity[]> {
    return httpGet(`/api/charts/activity${q({ guild_id: guildId ?? null, days })}`);
  },
  getTopUsers(guildId: string, limit = 10): Promise<TopUser[]> {
    return httpGet(`/api/stats/${guildId}/leaderboard${q({ limit })}`);
  },
};
