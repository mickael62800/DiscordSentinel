import { defineStore } from "pinia";
import { ref } from "vue";
import type { ComponentVisibilityEntry, RbacRole } from "@/types";
import { componentByKey, ROLE_RANK } from "@/rbac/componentRegistry";
import { rbacService } from "@/services/rbacService";
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
        // Charge visibility + myRole en parallele.
        const [list] = await Promise.all([
          rbacService
            .listComponentVisibility(guildId)
            .catch(() => [] as ComponentVisibilityEntry[]),
          myRoleStore.load(guildId),
        ]);
        overrides.value = list;
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
   *  - superadmin -> toujours true
   *  - override pour (role, key) -> sa valeur
   *  - sinon -> role >= minRole du registry
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
    const override = overrides.value.find(
      (o) => o.component_key === key && o.role === role,
    );
    if (override) return override.visible;
    return ROLE_RANK[role] >= ROLE_RANK[def.minRole];
  }

  return { overrides, loaded, loading, load, invalidate, visible };
});
