// Jeu Coude : supervision des joueurs.
//
// Lecture seule cote web. Les actions de jeu (combats, vols, primes, paris)
// restent sur Discord : ce sont des interactions entre joueurs, les rejouer
// depuis un back-office fausserait le jeu.

import { nexusGet } from "@/api/nexusHttp";

export interface CoudeProfile {
  guild_id: string;
  user_id: string;
  username: string;
  class: string;
  level: number;
  xp: number;
  atk: number;
  def: number;
  hp_current: number;
  hp_max: number;
  coins: number;
  stat_points: number;
  title: string;
  total_wins: number;
  total_losses: number;
  total_draws: number;
  total_stolen: number;
  cowardice_count: number;
  chaos_events: number;
}

export const nexusCoudeService = {
  /** GET /api/coude/{guild}/classement */
  ranking(guildId: string, limit = 50): Promise<CoudeProfile[]> {
    return nexusGet<CoudeProfile[]>(
      `/api/coude/${encodeURIComponent(guildId)}/classement?limit=${limit}`,
      guildId,
    );
  },
};
