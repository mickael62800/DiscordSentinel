// Types pour les features modération avancée Phase 5 :
// reminders, evidence, review, modstats.

// ── Reminders ──────────────────────────────────────────

export interface SanctionReminder {
  id: string;
  guild_id: string;
  moderator_id: string;
  moderator_name: string;
  target_id: string;
  target_name: string;
  action_type: string;
  reason: string;
  action_id: string;
  remind_at: string;
  expires_at: string;
  status: string;
  created_at: string;
}

export interface CreateReminderPayload {
  guild_id: string;
  moderator_id: string;
  moderator_name: string;
  target_id: string;
  target_name: string;
  action_type: string;
  reason: string;
  action_id: string;
  duration_secs: number;
  remind_before_secs?: number;
}

// ── Evidence ───────────────────────────────────────────

export interface EvidenceEntry {
  id: string;
  action_id: string;
  url: string;
  description: string | null;
  uploaded_by: string;
  uploaded_by_name: string;
  uploaded_at: string;
}

export interface AddEvidencePayload {
  action_id: string;
  url: string;
  description?: string | null;
  uploaded_by: string;
  uploaded_by_name: string;
}

// ── Review ─────────────────────────────────────────────

export interface ReviewQueueEntry {
  id: string;
  action_id: string;
  guild_id: string;
  added_by: string;
  added_by_name: string;
  reason: string | null;
  status: string; // "pending" | "approved" | "rejected" | "changed"
  reviewer_id: string | null;
  reviewer_name: string | null;
  reviewer_notes: string | null;
  added_at: string;
  resolved_at: string | null;
  action_type: string | null;
  target_name: string | null;
  action_reason: string | null;
}

export interface AddReviewPayload {
  action_id: string;
  guild_id: string;
  added_by: string;
  added_by_name: string;
  reason?: string | null;
}

export interface ResolveReviewPayload {
  status: "approved" | "rejected" | "changed";
  reviewer_id: string;
  reviewer_name: string;
  reviewer_notes?: string | null;
}

// ── Modstats ───────────────────────────────────────────

export interface ModStatsEntry {
  moderator_id: string;
  moderator_name: string;
  total: number;
  warns: number;
  mutes: number;
  bans: number;
  kicks: number;
}
