// Jeux joués depuis le site.
//
// Tout passe par `/api/me/games/*`, servi par sentinel-api. Aucun appel
// direct à nexus-api : ses routes portent le joueur dans leur chemin
// (`/api/wheel/{guild}/{user}/spin`), donc les exposer au navigateur
// laisserait n'importe qui jouer — ou dépenser — à la place d'un autre.
//
// Ni identifiant de serveur ni identifiant de joueur dans ces URL : le
// premier vient de la configuration, le second de la session Discord. Il n'y
// a rien à passer, et c'est exactement ce qui rend l'ensemble sûr.

import { httpGet, httpPost } from "@/api/http";

export interface Wallet {
  username: string;
  coins: number;
  total_earned: number;
  total_spent: number;
}

export interface Transaction {
  id: string;
  /// Négatif pour une dépense ou une perte.
  amount: number;
  balance_after: number;
  /// Origine technique (`wheel_payout`, `transfer`…). Sert à choisir l'icône.
  source: string;
  description: string;
  created_at: string;
}

export interface Rank {
  username: string;
  coins: number;
  rank: number;
  is_me: boolean;
}

export interface SpinResult {
  case_key: string;
  case_label: string;
  /// Gain ou perte en coins. 0 = case neutre.
  payout: number;
  balance_after: number;
  /// Résultat rare, à mettre en scène.
  is_memorable: boolean;
}

export const gamesService = {
  wallet(): Promise<Wallet> {
    return httpGet<Wallet>("/api/me/games/wallet");
  },

  history(limit = 15): Promise<Transaction[]> {
    return httpGet<Transaction[]>(`/api/me/games/history?limit=${limit}`);
  },

  leaderboard(limit = 10): Promise<Rank[]> {
    return httpGet<Rank[]>(`/api/me/games/leaderboard?limit=${limit}`);
  },

  /// Tire la Roue. Un tirage par jour et par personne, tous canaux confondus :
  /// avoir déjà tiré sur Discord fait échouer cet appel, et réciproquement.
  spinWheel(): Promise<SpinResult> {
    return httpPost<SpinResult>("/api/me/games/wheel/spin", {});
  },
};
