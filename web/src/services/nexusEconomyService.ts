// Economie de la plateforme Nexus : portefeuilles partages, classement et
// historique des transactions.
//
// Passe par la passerelle /nexus-api (autorisation `nexus.access` verifiee
// cote serveur par nginx).

import { nexusGet } from "@/api/nexusHttp";

export interface NexusWallet {
  guild_id: string;
  user_id: string;
  username: string;
  coins: number;
  total_earned: number;
  total_spent: number;
}

export interface NexusTransaction {
  id: string;
  amount: number;
  balance_after: number;
  source: string;
  description: string;
  reason: string | null;
  created_at: string;
}

export const nexusEconomyService = {
  /** GET /api/wallet/{guild}/leaderboard */
  leaderboard(guildId: string, limit = 20): Promise<NexusWallet[]> {
    return nexusGet<NexusWallet[]>(
      `/api/wallet/${encodeURIComponent(guildId)}/leaderboard?limit=${limit}`,
      guildId,
    );
  },

  /** GET /api/wallet/{guild}/{user} */
  wallet(guildId: string, userId: string): Promise<NexusWallet> {
    return nexusGet<NexusWallet>(
      `/api/wallet/${encodeURIComponent(guildId)}/${encodeURIComponent(userId)}`,
      guildId,
    );
  },

  /** GET /api/wallet/{guild}/{user}/history */
  history(guildId: string, userId: string, limit = 50): Promise<NexusTransaction[]> {
    return nexusGet<NexusTransaction[]>(
      `/api/wallet/${encodeURIComponent(guildId)}/${encodeURIComponent(userId)}/history?limit=${limit}`,
      guildId,
    );
  },
};
