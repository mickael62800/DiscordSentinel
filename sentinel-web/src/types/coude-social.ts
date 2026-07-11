// Vues admin sur les sous-systèmes social/toxic du Coude.
// Endpoints API réellement exposés : curses (malédictions) et primes (bounties)
// dans handlers/coude/{social,inventory}.rs. Les coalitions/vendettas avaient
// été planifiées mais jamais implémentées côté API — retirées de l'UI.

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

// Une prime (bounty) posée sur une cible. `GET /api/coude/{guild}/primes/{target}/active`
// renvoie la LISTE des primes actives (une par contributeur), pas un agrégat.
export interface Prime {
  id: string;
  guild_id: string;
  target_id: string;
  target_name: string;
  placed_by_id: string;
  placed_by_name: string;
  amount: number;
  claimed: boolean;
  claimed_by_id: string | null;
  claimed_by_name: string | null;
  claimed_at: string | null;
  created_at: string;
}
