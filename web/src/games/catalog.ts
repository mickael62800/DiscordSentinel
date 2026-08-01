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
  /// Jouable sur le site ? Faux = renvoie vers Discord.
  jouable: boolean;
}

export const GAMES: GameCard[] = [
  {
    key: "roue",
    nom: "La Roue du Destin",
    emoji: "🎡",
    pitch: "Un tirage par jour. Dix cases, de la ruine à la licorne.",
    couleur: "#a855f7",
    jouable: true,
  },
  {
    key: "coude",
    nom: "Coup de Coude",
    emoji: "💥",
    // Volontairement non jouable ici : l'intérêt du jeu est la réaction dans
    // le salon. Sur le web, on jouerait seul et le Discord se viderait.
    pitch: "Se joue sur Discord — tout le sel est dans la réaction des autres.",
    couleur: "#f39c12",
    jouable: false,
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
