// Service web pour Phase 5 : reminders, evidence, review, modstats.
// Endpoints API correspondants : voir
// services/api/src/adapters/inbound/http/handlers/{reminders.rs,moderation.rs}.

import { httpGet, httpPatch, httpPost } from "@/api/http";
import type {
  AddEvidencePayload,
  AddReviewPayload,
  CreateReminderPayload,
  EvidenceEntry,
  ModStatsEntry,
  ResolveReviewPayload,
  ReviewQueueEntry,
  SanctionReminder,
} from "@/types/moderation-advanced";

export const remindersService = {
  create(body: CreateReminderPayload): Promise<SanctionReminder> {
    return httpPost("/api/reminders", body);
  },
  listByGuild(guildId: string): Promise<SanctionReminder[]> {
    return httpGet(`/api/reminders/${guildId}`);
  },
  getPending(): Promise<SanctionReminder[]> {
    return httpGet(`/api/reminders/pending`);
  },
};

export const evidenceService = {
  list(actionId: string): Promise<EvidenceEntry[]> {
    return httpGet(`/api/moderation/evidence/${actionId}`);
  },
  add(body: AddEvidencePayload): Promise<EvidenceEntry> {
    return httpPost(`/api/moderation/evidence`, body);
  },
};

export const reviewService = {
  add(body: AddReviewPayload): Promise<ReviewQueueEntry> {
    return httpPost(`/api/moderation/review`, body);
  },
  listPending(guildId: string): Promise<ReviewQueueEntry[]> {
    return httpGet(`/api/moderation/review/${guildId}/pending`);
  },
  resolve(reviewId: string, body: ResolveReviewPayload): Promise<unknown> {
    return httpPatch(`/api/moderation/review/${reviewId}/resolve`, body);
  },
};

export interface ModstatsTrendDay {
  day: string;
  warns: number;
  mutes: number;
  bans: number;
  kicks: number;
}

export const modstatsService = {
  list(guildId: string, days = 30): Promise<ModStatsEntry[]> {
    return httpGet(`/api/moderation/modstats/${guildId}?days=${days}`);
  },
  trend(guildId: string, days = 30): Promise<ModstatsTrendDay[]> {
    return httpGet(`/api/moderation/modstats/${guildId}/trend?days=${days}`);
  },
};
