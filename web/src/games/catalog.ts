// Catalogue des jeux présentés dans le carrousel.
//
// Déclaratif : ajouter un jeu, c'est ajouter une entrée ici et son panneau
// dans la page. Le carrousel, la mémorisation et la navigation clavier
// n'ont pas à changer.
//
// Les jeux qui ne se jouent que sur Discord y figurent aussi, marqués comme
// tels. Les masquer laisserait croire qu'ils n'existent pas ; les montrer
// sans le dire enverrait cliquer dans le vide.

export interface GameCard {
  /// Identifiant stable : c'est lui qui est mémorisé, pas la position.
  /// Réordonner le catalogue ne doit pas changer le jeu retenu.
  key: string;
  nom: string;
  emoji: string;
  /// Une phrase, affichée sous le titre.
  pitch: string;
  /// Couleur d'accent de la vignette.
  couleur: string;
  /// Où le jeu se pratique. Affiché en badge sur la vignette : savoir AVANT
  /// de cliquer si une partie se lance ici ou sur Discord évite d'ouvrir un
  /// onglet pour rien.
  ///
  /// Remplace un booléen `jouable` : il ne distinguait pas « uniquement sur
  /// Discord » de « sur les deux », alors que c'est justement ce que le
  /// lecteur veut savoir.
  canaux: Canal[];
}

export type Canal = "discord" | "web";

const LIBELLES: Record<Canal, string> = {
  discord: "Discord",
  web: "Web",
};

/// Badge de la vignette : « Discord », « Web », ou « Discord / Web ».
export function badgeCanaux(jeu: GameCard): string {
  return jeu.canaux.map((c) => LIBELLES[c]).join(" / ");
}

export const GAMES: GameCard[] = [
  {
    key: "roue",
    nom: "La Roue du Destin",
    emoji: "🎡",
    pitch: "Un tirage par jour. Dix cases, de la ruine à la licorne.",
    couleur: "#a855f7",
    // Le même tirage des deux côtés : un seul quota quotidien, un seul
    // porte-monnaie.
    canaux: ["discord", "web"],
  },
  {
    key: "coude",
    nom: "Coup de Coude",
    emoji: "💥",
    // Les ACTIONS restent sur Discord — leur sel est la réaction du salon,
    // et les ouvrir ici le viderait. Mais tout ce qu'on y accomplit se
    // consulte sur le site : c'est ce que Discord fait mal, garder une trace
    // lisible d'un message qui a défilé.
    pitch: "Ta fiche, tes combats, ton inventaire. Les coups se donnent sur Discord.",
    couleur: "#f39c12",
    // Consultable ici, jouable seulement là-bas : le badge ne dit QUE où l'on
    // joue, sinon il promettrait une partie qui ne se lancera pas.
    canaux: ["discord"],
  },
];

const CLE_MEMOIRE = "ds.jeu_choisi";

/// Le jeu retenu du dernier passage, ou le premier du catalogue.
///
/// `localStorage` et non `sessionStorage` : le choix doit survivre à la
/// fermeture de l'onglet, c'est précisément ce qu'on attend d'un « je reviens
/// sur mon jeu ».
export function jeuMemorise(): string {
  try {
    const stocke = localStorage.getItem(CLE_MEMOIRE);
    // Un jeu retiré du catalogue ne doit pas laisser la page vide.
    if (stocke && GAMES.some((g) => g.key === stocke)) return stocke;
  } catch {
    /* stockage indisponible : on retombe sur le premier jeu */
  }
  return GAMES[0].key;
}

export function memoriserJeu(key: string): void {
  try {
    localStorage.setItem(CLE_MEMOIRE, key);
  } catch {
    /* idem : la mémorisation est un confort, pas une exigence */
  }
}
