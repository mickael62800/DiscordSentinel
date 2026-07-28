// Types Phase 10 — pages polish (sponsorships, temp-roles, system).

export interface Sponsorship {
  id: string;
  guild_id: string;
  sponsor_id: string;
  sponsored_id: string;
  created_at: string;
}

export interface TempRole {
  id: string;
  guild_id: string;
  user_id: string;
  role_id: string;
  expires_at: string;
  created_at: string;
}

export interface ModelInfo {
  name: string;
  model_type: string;
  loaded: boolean;
}

export interface ModelsStatusResponse {
  models: ModelInfo[];
}

export interface CacheStats {
  hits: number;
  misses: number;
  total: number;
  hit_rate_percent: number;
}

export interface CreateSponsorshipPayload {
  guild_id: string;
  sponsor_id: string;
  sponsored_id: string;
}

export interface CreateTempRolePayload {
  guild_id: string;
  user_id: string;
  role_id: string;
  expires_at: string; // RFC3339
}
