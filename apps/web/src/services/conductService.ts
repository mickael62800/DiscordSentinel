import { httpGet, httpPost } from "@/api/http";
import type { ConductConfig, UserConductPoints, ConductPointsLog } from "@/types";

export interface SaveConductConfigPayload {
  guild_id: string;
  max_points?: number;
  regen_amount?: number;
  regen_interval?: string;
  penalty_warn?: number;
  penalty_delete?: number;
  penalty_mute?: number;
  penalty_ban?: number;
}

export const conductService = {
  getConfig(guildId: string): Promise<ConductConfig> {
    return httpGet(`/api/conduct/config/${guildId}`);
  },
  saveConfig(body: SaveConductConfigPayload): Promise<ConductConfig> {
    return httpPost(`/api/conduct/config`, body);
  },
  runRegenTick(): Promise<unknown> {
    return httpPost(`/api/conduct/regen-tick`);
  },
  syncBanProposals(): Promise<{ created: number }> {
    return httpPost(`/api/conduct/sync-ban-proposals`);
  },
  getLeaderboard(guildId: string): Promise<UserConductPoints[]> {
    return httpGet(`/api/conduct/${guildId}/leaderboard`);
  },
  getPoints(guildId: string, userId: string): Promise<UserConductPoints> {
    return httpGet(`/api/conduct/${guildId}/${userId}`);
  },
  getLog(guildId: string, userId: string): Promise<ConductPointsLog[]> {
    return httpGet(`/api/conduct/${guildId}/${userId}/log`);
  },
  adjustPoints(guildId: string, userId: string, amount: number, reason: string): Promise<unknown> {
    return httpPost(`/api/conduct/${guildId}/${userId}/add`, { amount, reason });
  },
};
