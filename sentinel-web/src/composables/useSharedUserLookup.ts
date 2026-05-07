import { ref } from "vue";

// State module-level partage : tous les `useSharedUserLookup()` lisent
// et ecrivent la meme ref. Permet a NotesPage + EvidencePage (montees
// cote a cote dans le sous-onglet "Notes & Preuves") de partager le
// meme champ ID utilisateur. Le bouton du Journal le set aussi pour
// pre-remplir le lookup au switch d'onglet.
const sharedUserId = ref("");

export function useSharedUserLookup() {
  return { sharedUserId };
}
