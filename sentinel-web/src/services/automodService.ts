import { httpGet, httpPost } from "@/api/http";
import type { Infraction } from "@/types";
import { q } from "./_query";

export interface AutomodReview {
  id: string;
  guild_id: string;
  channel_id: string;
  message_id: string;
  user_id: string;
  user_name: string;
  content_preview: string;
  suggested_action: "warn" | "delete" | "mute" | "ban";
  score: number;
  reason: string;
  flags: Record<string, boolean>;
  status: "pending" | "applied" | "ignored";
  applied_action: string | null;
  resolved_by_id: string | null;
  resolved_by_name: string | null;
  resolved_source: string | null;
  created_at: string;
  resolved_at: string | null;
}

export type ResolveActionChoice = "warn" | "delete" | "mute" | "ban" | "ignore";

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
  /** GET /api/automod/{guild_id}/reviews — cartes pending par defaut. */
  listReviews(
    guildId: string,
    params: { include_resolved?: boolean; limit?: number } = {},
  ): Promise<AutomodReview[]> {
    return httpGet(
      `/api/automod/${guildId}/reviews${q({
        include_resolved: params.include_resolved ?? null,
        limit: params.limit ?? null,
      })}`,
    );
  },
  /** POST /api/automod/reviews/{review_id}/resolve — applique une action. */
  resolveReview(
    reviewId: string,
    body: {
      applied_action: ResolveActionChoice;
      resolved_by_id: string;
      resolved_by_name: string;
    },
  ): Promise<AutomodReview> {
    return httpPost(`/api/automod/reviews/${reviewId}/resolve`, body);
  },
};
