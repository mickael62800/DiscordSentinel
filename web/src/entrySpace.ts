// Espace visé lors de la connexion : membre ou administration.
//
// Le site a deux publics distincts qui passent par la MÊME connexion Discord.
// Ce qui les sépare, c'est la destination après authentification — d'où cette
// intention, mémorisée avant de partir vers Discord et relue au retour.
//
// `sessionStorage` et non `localStorage` : l'intention ne vaut que pour cette
// connexion-ci. Un ancien choix ne doit pas détourner une visite ultérieure.

export type EntrySpace = "membre" | "admin";

const STORAGE_KEY = "ds.entry_space";

/// Destination finale de chaque espace.
const DESTINATIONS: Record<EntrySpace, string> = {
  membre: "/membre",
  admin: "/dashboard",
};

export function rememberEntrySpace(value: unknown): void {
  if (value === "membre" || value === "admin") {
    try {
      sessionStorage.setItem(STORAGE_KEY, value);
    } catch {
      /* stockage indisponible (navigation privée stricte) : on ignore, le
         repli sur le tableau de bord reste correct. */
    }
  }
}

/// Consomme l'intention et renvoie la route de destination. Sans intention
/// mémorisée, on retombe sur l'administration : c'était le comportement
/// historique, et c'est la destination attendue d'un lien direct vers /login.
export function takeEntryDestination(): string {
  let stored: string | null = null;
  try {
    stored = sessionStorage.getItem(STORAGE_KEY);
    sessionStorage.removeItem(STORAGE_KEY);
  } catch {
    /* idem */
  }
  return stored === "membre" ? DESTINATIONS.membre : DESTINATIONS.admin;
}
