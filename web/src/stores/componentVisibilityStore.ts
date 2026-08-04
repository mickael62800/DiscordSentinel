import { defineStore } from "pinia";
import { ref } from "vue";
import type { ComponentVisibilityEntry } from "@/types";

/**
 * Store Pinia : visibilite des composants d'interface.
 *
 * Le back-office n'a plus qu'un seul utilisateur possible (les Discord user
 * IDs de SUPERADMIN_USER_IDS), donc plus rien a masquer : les overrides par
 * role et les gates `min_role` par composant ont ete supprimes, cote base
 * comme cote API.
 *
 * Le store est conserve plutot que supprime avec ses sites d'appel : `visible`
 * garde sa signature et repond desormais toujours `true`. Plus aucun appel
 * reseau n'est fait ici — les endpoints correspondants n'existent plus.
 */
export const useComponentVisibilityStore = defineStore("componentVisibility", () => {
  const overrides = ref<ComponentVisibilityEntry[]>([]);
  const loaded = ref(true);
  const loading = ref(false);

  async function load(_guildId: string): Promise<void> {
    /* Plus rien a charger. */
  }

  function invalidate(): void {
    /* Aucun cache distant a invalider. */
  }

  function visible(_key: string): boolean {
    return true;
  }

  return { overrides, loaded, loading, load, invalidate, visible };
});
