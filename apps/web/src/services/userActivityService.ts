import { httpGet } from "@/api/http";
import { q } from "./_query";

export interface UserActivity {
  id: string;
  guild_id: string;
  user_id: string;
  event_type: string;
  channel_id: string | null;
  channel_name: string | null;
  content: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
}

export const userActivityService = {
  list(
    guildId: string,
    userId: string,
    opts?: { eventType?: string; limit?: number; offset?: number },
  ): Promise<UserActivity[]> {
    return httpGet(
      `/api/user-activity/${guildId}/${userId}${q({
        event_type: opts?.eventType,
        limit: opts?.limit,
        offset: opts?.offset,
      })}`,
    );
  },
};
