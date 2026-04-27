import { httpGet } from "@/api/http";
import type { Infraction } from "@/types";
import { q } from "./_query";

export const automodService = {
  /** GET /api/automod/{guild_id}/detections — timeline filtree action='detection'. */
  listDetections(
    guildId: string,
    params: { user_id?: string; limit?: number; offset?: number } = {},
  ): Promise<Infraction[]> {
    return httpGet(
      `/api/automod/${guildId}/detections${q({
        user_id: params.user_id ?? null,
        limit: params.limit ?? null,
        offset: params.offset ?? null,
      })}`,
    );
  },
};
