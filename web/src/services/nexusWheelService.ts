// Cases de la Roue du Destin, par serveur.
//
// Absence de personnalisation = roue historique : l'API renvoie alors les dix
// cases d'origine avec `customized: false`. C'est ce drapeau qui permet de
// distinguer « ce serveur a choisi exactement la roue d'origine » de « ce
// serveur n'a rien choisi » — sans lui, on ne pourrait pas proposer de
// revenir en arrière.

import { nexusGet, nexusPut } from "@/api/nexusHttp";

export interface WheelCase {
  /// Identifiant stable, utilisé par le site pour retrouver le secteur tiré.
  key: string;
  label: string;
  /// Négatif = perte, 0 = case blanche.
  payout: number;
  /// Poids de tirage relatif. Au moins 1.
  weight: number;
}

export interface WheelCases {
  cases: WheelCase[];
  customized: boolean;
}

export const nexusWheelService = {
  list(guildId: string): Promise<WheelCases> {
    return nexusGet<WheelCases>(
      `/api/wheel/${encodeURIComponent(guildId)}/cases`,
      guildId,
    );
  },
  /** Remplace INTÉGRALEMENT la roue. Une liste vide restaure celle d'origine. */
  replace(guildId: string, cases: WheelCase[]): Promise<WheelCases> {
    return nexusPut<WheelCases>(
      `/api/wheel/${encodeURIComponent(guildId)}/cases`,
      guildId,
      { cases },
    );
  },
};

/// Part de chance d'une case, en pourcentage du total des poids.
///
/// C'est la seule lecture utile d'un poids : « 3 » ne veut rien dire seul,
/// « 2,8 % » se compare à une intuition.
export function chancePercent(cases: WheelCase[], weight: number): number {
  const total = cases.reduce((s, c) => s + Math.max(0, c.weight), 0);
  if (total <= 0) return 0;
  return (weight / total) * 100;
}
