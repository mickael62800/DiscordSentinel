import { httpGet, httpPost } from "@/api/http";
import type { ConductConfig, UserConductPoints, ConductPointsLog } from "@/types";

export const conductService = {
  getConfig(guildId: string): Promise<ConductConfig> {
    return httpGet(`/api/conduct/config/${guildId}`);
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
