export interface StrikeThreshold {
  strikes: number;
  action: string; // "warn" | "mute" | "ban" | "kick"
  duration: number | null; // seconds (mute/ban temporaire)
}

export interface StrikeConfig {
  guild_id: string;
  window_secs: number;
  thresholds: StrikeThreshold[];
  enabled: boolean;
}

export interface UserStrike {
  id: string;
  guild_id: string;
  user_id: string;
  reason: string;
  source: string;
  infraction_id: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface AddStrikePayload {
  guild_id: string;
  user_id: string;
  reason: string;
  source: string;
  infraction_id?: string | null;
}

export interface StrikeResult {
  active_count: number;
  escalation_action: string | null;
  escalation_duration: number | null;
}

export interface SaveStrikeConfigPayload {
  window_secs: number;
  thresholds: StrikeThreshold[];
  enabled: boolean;
}
