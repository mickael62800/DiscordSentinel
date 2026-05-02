import { defineStore } from "pinia";
import { ref } from "vue";
import type { ComponentVisibilityEntry, RbacRole } from "@/types";
import { componentByKey, ROLE_RANK } from "@/rbac/componentRegistry";
import { rbacService, type ComponentMinRoleEntry } from "@/services/rbacService";
import { useMyRoleStore } from "./myRoleStore";

/**
 * Store Pinia : overrides de visibilite des composants UI par role
 * (configurable depuis la page RBAC, gate par owner+).
 *
 * Charge en parallele myRole (via myRoleStore.load) car les 2 sont toujours
 * utilisees ensemble pour resoudre la visibilite.
 *
 * Visible dans Vue DevTools sous "componentVisibility".
 */
export const useComponentVisibilityStore = defineStore("componentVisibility", () => {
  const overrides = ref<ComponentVisibilityEntry[]>([]);
  /** Gates RBAC granulaires (purge/reset) — synchronises avec ce que l'API
   *  applique reellement (table rbac_component_min_role). */
  const minRoles = ref<ComponentMinRoleEntry[]>([]);
  const loaded = ref(false);
  const loading = ref(false);

  let lastLoadedGuild: string | null = null;
  let inFlight: Promise<void> | null = null;

  async function load(guildId: string): Promise<void> {
    if (lastLoadedGuild === guildId && !inFlight) return;
    if (inFlight && lastLoadedGuild === guildId) return inFlight;

    const myRoleStore = useMyRoleStore();

    loading.value = true;
    inFlight = (async () => {
      try {
        // Charge visibility + myRole + min_roles en parallele.
        const [vis, minRolesList] = await Promise.all([
          rbacService
            .listComponentVisibility(guildId)
            .catch(() => [] as ComponentVisibilityEntry[]),
          rbacService
            .listComponentMinRoles(guildId)
            .catch(() => [] as ComponentMinRoleEntry[]),
          myRoleStore.load(guildId),
        ]);
        overrides.value = vis;
        minRoles.value = minRolesList;
        lastLoadedGuild = guildId;
        loaded.value = true;
      } finally {
        loading.value = false;
        inFlight = null;
      }
    })();
    return inFlight;
  }

  function invalidate(): void {
    lastLoadedGuild = null;
    loaded.value = false;
  }

  /**
   * Resout la visibilite d'un composant pour le role courant.
   *
   * Priorite (ordre):
   *  1. superadmin -> toujours true (bypass complet)
   *  2. min_role API gate (rbac_component_min_role) -> source de verite
   *     securite. Si le user n'a pas le role minimum, false.
   *  3. override visibility UI (rbac_component_visibility) -> peut cacher
   *     un bouton meme si le user a le role.
   *  4. registry default (componentRegistry.minRole)
   */
  function visible(key: string): boolean {
    const myRoleStore = useMyRoleStore();
    const isSuper = myRoleStore.isSuper;
    const role = myRoleStore.role as RbacRole | null;

    if (isSuper) return true;
    if (!role) return false;
    const def = componentByKey(key);
    if (!def) {
      console.warn(`[visibility] composant inconnu: ${key}`);
      return true; // failsafe
    }

    // 1. Gate API : si le composant est protege par min_role (purge/reset),
    //    on utilise l'effective_role retourne par l'API. Sinon registry.minRole.
    const gate = minRoles.value.find((m) => m.component_key === key);
    const minRole = gate ? gate.effective_role : def.minRole;
    if (ROLE_RANK[role] < ROLE_RANK[minRole]) return false;

    // 2. Override visibility (peut cacher davantage, jamais elargir).
    const visOverride = overrides.value.find(
      (o) => o.component_key === key && o.role === role,
    );
    if (visOverride) return visOverride.visible;
    return true;
  }

  return { overrides, minRoles, loaded, loading, load, invalidate, visible };
});
