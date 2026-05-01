import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { MyRole, RbacRole } from "@/types";
import { rbacService } from "@/services/rbacService";

/**
 * Store Pinia : role RBAC de l'utilisateur courant pour la guild selectionnee.
 *
 * Load idempotent : si deja charge pour cette guild, no-op. Si en cours de
 * chargement, retourne la promesse en vol (evite les fetch paralleles).
 *
 * Visible dans Vue DevTools sous "myRole".
 */
export const useMyRoleStore = defineStore("myRole", () => {
  const myRole = ref<MyRole | null>(null);
  const loading = ref(false);

  // Etats internes (pas exposes - juste pour le caching)
  let lastLoadedGuild: string | null = null;
  let inFlight: Promise<MyRole | null> | null = null;

  const role = computed<RbacRole | null>(() => myRole.value?.role ?? null);
  const isSuper = computed(() => myRole.value?.is_superadmin === true);

  async function load(guildId: string): Promise<MyRole | null> {
    if (lastLoadedGuild === guildId && !inFlight) return myRole.value;
    if (inFlight && lastLoadedGuild === guildId) return inFlight;

    loading.value = true;
    inFlight = (async () => {
      try {
        const me = await rbacService.getMyRole(guildId).catch(() => null);
        myRole.value = me;
        lastLoadedGuild = guildId;
        return me;
      } finally {
        loading.value = false;
        inFlight = null;
      }
    })();
    return inFlight;
  }

  function reset(): void {
    myRole.value = null;
    lastLoadedGuild = null;
  }

  function invalidate(): void {
    lastLoadedGuild = null;
  }

  return { myRole, role, isSuper, loading, load, reset, invalidate };
});
