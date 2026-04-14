import { httpGet, httpPost, httpDelete } from "@/api/http";
import type { LevelConfig, UserLevel, LevelReward } from "@/types";
import { q } from "./_query";

export const levelsService = {
  getConfig(guildId: string): Promise<LevelConfig> {
    return httpGet(`/api/levels/config/${guildId}`);
  },
  getLeaderboard(guildId: string): Promise<UserLevel[]> {
    return httpGet(`/api/levels/${guildId}/leaderboard`);
  },
  getRewards(guildId: string): Promise<LevelReward[]> {
    return httpGet(`/api/levels/rewards/${guildId}`);
  },
  setReward(guildId: string, level: number, roleId: string, source: string): Promise<unknown> {
    return httpPost("/api/levels/rewards", { guild_id: guildId, level, role_id: roleId, source });
  },
  deleteReward(guildId: string, level: number, source: string): Promise<unknown> {
    return httpDelete(`/api/levels/rewards/${guildId}/${level}${q({ source })}`);
  },
};
