import { computed, ref, watch } from "vue";
import type { MyRole, RbacRole } from "@/types";
import { rbacService } from "@/services/rbacService";
import { useGuildSelector } from "./useGuildSelector";

/**
 * Singleton du role RBAC de l'utilisateur courant.
 *
 * Avant : useRbac et useComponentVisibility appelaient chacun /api/rbac/me/{guild}
 * a leur init -> 2 requetes pour la meme donnee. Maintenant : 1 seul appel
 * partage en module-scope, refetch uniquement quand selectedGuildId change.
 */

const myRole = ref<MyRole | null>(null);
const loading = ref(false);
let lastLoadedGuild: string | null = null;
let inFlight: Promise<MyRole | null> | null = null;

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

export async function preloadMyRole(guildId: string): Promise<void> {
  await load(guildId);
}

export function useMyRole() {
  const { selectedGuildId } = useGuildSelector();

  watch(
    selectedGuildId,
    (gid) => {
      if (gid && gid !== lastLoadedGuild) {
        lastLoadedGuild = null;
        void load(gid);
      } else if (!gid) {
        myRole.value = null;
        lastLoadedGuild = null;
      }
    },
    { immediate: true },
  );

  const role = computed<RbacRole | null>(() => myRole.value?.role ?? null);
  const isSuper = computed(() => myRole.value?.is_superadmin === true);

  async function reload(): Promise<void> {
    if (selectedGuildId.value) {
      lastLoadedGuild = null;
      await load(selectedGuildId.value);
    }
  }

  return { myRole, role, isSuper, loading, reload };
}
