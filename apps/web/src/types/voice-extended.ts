// Types pour Phase 6 — voice channels CRUD complet (themes,
// whitelists, bans, invites, transfer, co-admins).
// Endpoints : services/api/src/adapters/inbound/http/handlers/voice_channels.rs

export interface VoiceChannelTheme {
  id: string;
  guild_id: string;
  name: string;
  emoji: string | null;
  channel_name_template: string;
  member_limit: number | null;
  visibility: string; // "public" | "private" | "muted"
  locked: boolean;
  queue_enabled: boolean;
  bitrate: number | null;
  slowmode_secs: number | null;
  stage_enabled: boolean;
  is_default: boolean;
  sort_order: number;
  created_at: string;
}

export interface CreateThemePayload {
  name: string;
  emoji?: string | null;
  channel_name_template?: string;
  member_limit?: number | null;
  visibility?: string;
  locked?: boolean;
  queue_enabled?: boolean;
  bitrate?: number | null;
  slowmode_secs?: number | null;
  stage_enabled?: boolean;
  is_default?: boolean;
  sort_order?: number;
}

export type UpdateThemePayload = Partial<CreateThemePayload>;

// ── Whitelists / Bans / Invites / Co-admins ────────────────

export interface WhitelistEntry {
  id: string;
  owner_id: string;
  target_id: string;
  target_name: string;
  created_at: string;
}

export interface AddWhitelistPayload {
  guild_id: string;
  owner_id: string;
  target_id: string;
  target_name: string;
}

export interface ChannelBan {
  id: string;
  channel_id: string;
  user_id: string;
  user_name: string;
  banned_by: string;
  reason: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface BanFromChannelPayload {
  user_id: string;
  user_name: string;
  banned_by: string;
  reason?: string | null;
  duration_secs?: number | null;
}

export interface InviteLink {
  id: string;
  channel_id: string;
  guild_id: string;
  created_by: string;
  created_by_name: string;
  code: string;
  max_uses: number | null;
  current_uses: number;
  expires_at: string | null;
  created_at: string;
}

export interface CreateInvitePayload {
  created_by: string;
  created_by_name: string;
  duration_secs?: number | null;
  max_uses?: number | null;
}

export interface CoAdmin {
  id: string;
  user_id: string;
  user_name: string;
  granted_at: string;
}

export interface AddCoAdminPayload {
  user_id: string;
  user_name: string;
}

export interface TransferOwnershipPayload {
  new_owner_id: string;
  new_owner_name: string;
}
