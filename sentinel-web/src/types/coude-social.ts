// Types Phase 8 — vues admin sur les sous-systèmes social/toxic du Coude.
// Endpoints API : sentinel-api/src/adapters/inbound/http/handlers/coude/{curses,bounty,coalition,vendetta}.rs

export interface ActiveCurse {
  id: string;
  guild_id: string;
  target_id: string;
  source_id: string;
  kind: string;
  kind_label: string;
  kind_emoji: string;
  created_at: string;
  expires_at: string;
  lifted_at: string | null;
  lifted_by: string | null;
}

export interface ActiveBounty {
  id: string;
  guild_id: string;
  target_id: string;
  total_amount: number;
  status: string;
  opened_at: string;
  claimed_by: string | null;
  claimed_at: string | null;
}

export interface CoalitionMember {
  member_id: string;
  member_name: string;
  joined_at: string;
}

export interface ActiveCoalition {
  id: string;
  guild_id: string;
  target_id: string;
  opened_at: string;
  expires_at: string;
  status: string;
  broken_by: string | null;
  broken_at: string | null;
  members: CoalitionMember[];
}

export interface ActiveVendetta {
  id: string;
  guild_id: string;
  challenger_id: string;
  target_id: string;
  declared_at: string;
  expires_at: string;
  status: string;
  resolved_at: string | null;
}
