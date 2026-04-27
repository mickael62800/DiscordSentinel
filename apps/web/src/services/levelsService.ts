import { httpGet, httpPost, httpDelete } from "@/api/http";
import type { LevelConfig, UserLevel, LevelReward } from "@/types";
import { q } from "./_query";

export interface SaveLevelConfigPayload {
  guild_id: string;
  xp_per_message?: number;
  xp_per_voice_minute?: number;
  xp_cooldown_secs?: number;
  level_up_channel_id?: string | null;
  level_up_message?: string;
  excluded_channels?: string[];
  enabled?: boolean;
}

export interface AddXpPayload {
  guild_id: string;
  user_id: string;
  username: string;
  amount: number;
  source?: "text" | "voice";
}

export const levelsService = {
  getConfig(guildId: string): Promise<LevelConfig> {
    return httpGet(`/api/levels/config/${guildId}`);
  },
  saveConfig(body: SaveLevelConfigPayload): Promise<LevelConfig> {
    return httpPost(`/api/levels/config`, body);
  },
  addXp(body: AddXpPayload): Promise<unknown> {
    return httpPost(`/api/levels/xp`, body);
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
