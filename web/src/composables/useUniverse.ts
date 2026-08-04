// Univers applicatif courant : Sentinel (moderation/communaute) ou Nexus
// (plateforme jeux). Deux produits distincts qui partagent le meme dashboard,
// la meme identite Discord et le meme RBAC.
//
// L'univers n'est PAS un droit : il ne fait que filtrer la navigation. Le
// droit d'acceder a Nexus est porte par le gate RBAC `nexus.access`, verifie
// cote serveur par la passerelle nginx avant chaque appel a nexus-api.

import { ref, watch } from "vue";
import { useRoute } from "vue-router";
import type { Universe } from "@/composables/useDashboardSections";

const STORAGE_KEY = "ds.universe";

function readStored(): Universe {
  return localStorage.getItem(STORAGE_KEY) === "nexus" ? "nexus" : "sentinel";
}

// Etat partage par tous les composants (module scope, pas de store Pinia
// necessaire : une seule valeur, pas de chargement asynchrone).
const current = ref<Universe>(readStored());

export function useUniverse() {
  const route = useRoute();

  // Le back-office n'a plus qu'un seul utilisateur possible (superadmin), qui
  // voit tout : les deux univers sont toujours accessibles. `canAccessNexus`
  // est conserve dans l'API pour ne pas toucher TopBar, mais vaut toujours vrai.
  const canAccessNexus = true;

  /// L'URL fait foi : arriver sur /nexus/... bascule l'univers, ce qui evite
  /// une barre laterale incoherente avec la page affichee (lien direct,
  /// favori, rechargement).
  watch(
    () => route.path,
    (path) => {
      const target: Universe = path.startsWith("/nexus") ? "nexus" : "sentinel";
      if (target !== current.value) current.value = target;
    },
    { immediate: true },
  );

  watch(current, (u) => localStorage.setItem(STORAGE_KEY, u));

  function setUniverse(u: Universe) {
    current.value = u;
  }

  return { universe: current, canAccessNexus, setUniverse };
}
