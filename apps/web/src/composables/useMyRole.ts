import { storeToRefs } from "pinia";
import { watch } from "vue";
import { useMyRoleStore } from "@/stores/myRoleStore";
import { useGuildSelector } from "./useGuildSelector";

/**
 * Wrapper composable : delegue au store Pinia myRole + integre le watch
 * sur selectedGuildId pour recharger automatiquement quand l'utilisateur
 * change de guild. API publique identique a la version singleton precedente.
 */
export function useMyRole() {
  const store = useMyRoleStore();
  const { selectedGuildId } = useGuildSelector();
  const { myRole, role, isSuper, loading } = storeToRefs(store);

  watch(
    selectedGuildId,
    (gid) => {
      if (gid) void store.load(gid);
      else store.reset();
    },
    { immediate: true },
  );

  async function reload(): Promise<void> {
    if (selectedGuildId.value) {
      store.invalidate();
      await store.load(selectedGuildId.value);
    }
  }

  return { myRole, role, isSuper, loading, reload };
}

/** Helper appele depuis useAppInit (router.beforeEach) pour prefetch. */
export async function preloadMyRole(guildId: string): Promise<void> {
  await useMyRoleStore().load(guildId);
}
