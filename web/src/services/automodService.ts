import { httpGet, httpPost } from "@/api/http";
import type { Infraction } from "@/types";
import { q } from "./_query";

export interface AutomodIncident {
  message_id?: string;
  channel_id?: string;
  content_preview?: string;
  score?: number;
  reason?: string;
  suggested_action?: string;
  at?: string;
}

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
  status: "pending" | "voting" | "decided" | "applied" | "ignored";
  applied_action: string | null;
  resolved_by_id: string | null;
  resolved_by_name: string | null;
  resolved_source: string | null;
  created_at: string;
  resolved_at: string | null;
  // Agrégation par utilisateur (anti-flood) + vote
  incident_count: number;
  cumulative_score: number;
  /** Détail des incidents agrégés (affiché dans le dashboard, plus sur la carte Discord). */
  incidents: AutomodIncident[];
  voting_deadline: string | null;
  decided_action: string | null;
  quorum_met: boolean;
  // Salon de discussion lié (si ouvert depuis la carte)
  discussion_channel_id: string | null;
}

export type ResolveActionChoice = "prevention" | "warn" | "delete" | "mute" | "ban" | "ignore";

/** Un message du transcript du salon de discussion (trace persistante). */
export interface DiscussionMessage {
  discord_message_id: string;
  author_id: string;
  author_name: string;
  author_is_bot: boolean;
  content: string;
  sent_at: string;
}

/** Une entree de stat de faux positifs (globale, par flag ou par action). */
export interface FpBucket {
  total: number;
  overturned: number;
  ignored: number;
  fp_rate: number;
}

export interface FpFlagStat extends FpBucket {
  flag: string;
}

export interface FpActionStat extends FpBucket {
  suggested_action: string;
}

/** Reponse de GET /api/automod/{guild_id}/fp-stats. */
export interface FpStats {
  days: number;
  capped: boolean;
  overall: FpBucket;
  by_flag: FpFlagStat[];
  by_suggested_action: FpActionStat[];
}

export const automodService = {
  /** GET /api/automod/{guild_id}/fp-stats — taux de faux positifs de l'automod. */
  fpStats(guildId: string, days = 30): Promise<FpStats> {
    return httpGet(`/api/automod/${guildId}/fp-stats${q({ days })}`);
  },
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
  /** GET /api/automod/reviews/{review_id}/discussion/messages — transcript (trace). */
  getDiscussionMessages(reviewId: string): Promise<DiscussionMessage[]> {
    return httpGet(`/api/automod/reviews/${reviewId}/discussion/messages`);
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
