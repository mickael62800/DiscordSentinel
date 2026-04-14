import { httpGet, httpPost } from "@/api/http";
import type { ConfirmedBan, ModerationActionResponse, UserModerationHistory } from "@/types";
import { q } from "./_query";

export interface LogActionParams {
  guildId: string;
  channelId: string;
  moderatorId: string;
  moderatorName: string;
  targetId: string;
  targetName: string;
  actionType: string;
  reason: string;
  gravity?: string;
  duration?: number;
}

export const moderationService = {
  executeBan(guildId: string, userId: string, reason: string): Promise<unknown> {
    return httpPost("/api/moderation/execute-ban", { guild_id: guildId, user_id: userId, reason });
  },
  executeUnban(guildId: string, userId: string): Promise<unknown> {
    return httpPost("/api/moderation/execute-unban", { guild_id: guildId, user_id: userId });
  },
  getConfirmedBans(guildId?: string | null): Promise<ConfirmedBan[]> {
    return httpGet(`/api/moderation/bans${q({ guild_id: guildId ?? null })}`);
  },
  getHistory(guildId: string, userId: string): Promise<UserModerationHistory> {
    return httpGet(`/api/moderation/history/${guildId}/${userId}`);
  },
  logAction(params: LogActionParams): Promise<ModerationActionResponse> {
    return httpPost("/api/moderation/actions", {
      guild_id: params.guildId,
      channel_id: params.channelId,
      moderator_id: params.moderatorId,
      moderator_name: params.moderatorName,
      target_id: params.targetId,
      target_name: params.targetName,
      action_type: params.actionType,
      reason: params.reason,
      gravity: params.gravity,
      duration: params.duration,
    });
  },
};
