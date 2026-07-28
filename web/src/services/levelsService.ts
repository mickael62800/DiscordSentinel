import { httpGet, httpPost } from "@/api/http";
import type { UserLevel } from "@/types";

export interface AddXpPayload {
  guild_id: string;
  user_id: string;
  username: string;
  amount: number;
  source?: "text" | "voice";
}

export const levelsService = {
  addXp(body: AddXpPayload): Promise<unknown> {
    return httpPost(`/api/levels/xp`, body);
  },
  getLeaderboard(guildId: string): Promise<UserLevel[]> {
    return httpGet(`/api/levels/${guildId}/leaderboard`);
  },
  /** Admin override : set valeur exacte XP texte/voix (champs Option). */
  setUserXp(body: {
    guild_id: string;
    user_id: string;
    xp_text?: number;
    xp_voice?: number;
  }): Promise<UserLevel> {
    return httpPost("/api/levels/admin/set-xp", body);
  },
  /** Admin override : reset XP a 0 (target = "all" / "text" / "voice"). */
  resetUserXp(body: {
    guild_id: string;
    user_id: string;
    target: "all" | "text" | "voice";
  }): Promise<UserLevel> {
    return httpPost("/api/levels/admin/reset-xp", body);
  },
};
