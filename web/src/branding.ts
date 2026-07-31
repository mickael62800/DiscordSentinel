// Identites visuelles du site — source unique.
//
// Trois marques distinctes cohabitent :
//   - COMMUNITY : la communaute elle-meme (La Bande du Canape). C'est la
//     marque du site public et de la page de connexion : un visiteur arrive
//     chez la communaute, pas chez un outil d'administration.
//   - SENTINEL  : le back-office moderation / communaute.
//   - NEXUS     : la plateforme jeux.
//
// Regrouper les chemins ici evite les references en dur dispersees dans les
// composants — c'est precisement ce qui avait laisse un `/logo.png`
// inexistant reference a trois endroits + le favicon.

export interface Brand {
  /// Nom affiche.
  name: string;
  /// Chemin du logo, servi depuis `web/public/`.
  logo: string;
  /// Phrase d'accroche, utilisee sous le titre.
  tagline: string;
}

export const COMMUNITY: Brand = {
  name: "La Bande du Canape",
  // Fourni par la communaute. Si le fichier est absent, `onLogoError` masque
  // proprement l'image plutot que d'afficher une icone cassee.
  logo: "/canape_logo.png",
  tagline: "Evenements, jeux, classements — la vie du serveur.",
};

export const SENTINEL: Brand = {
  name: "Sentinel",
  logo: "/sentinel_logo.png",
  tagline: "Moderation et communaute",
};

export const NEXUS: Brand = {
  name: "Nexus",
  logo: "/nexus_logo.png",
  tagline: "Plateforme jeux",
};

/// Masque l'image si le fichier n'existe pas encore (logo pas encore fourni).
/// Sans ca, le navigateur affiche une icone de lien casse.
export function onLogoError(event: Event): void {
  const el = event.target as HTMLImageElement | null;
  if (el) el.style.display = "none";
}
