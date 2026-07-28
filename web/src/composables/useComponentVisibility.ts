import { storeToRefs } from "pinia";
import { watch } from "vue";
import { useComponentVisibilityStore } from "@/stores/componentVisibilityStore";
import { useMyRoleStore } from "@/stores/myRoleStore";
import { useGuildSelector } from "./useGuildSelector";

/**
 * Wrapper composable : delegue aux stores Pinia componentVisibility +
 * myRole, integre le watch sur selectedGuildId. API publique identique
 * a la version singleton precedente.
 */
export function useComponentVisibility() {
  const store = useComponentVisibilityStore();
  const myRoleStore = useMyRoleStore();
  const { selectedGuildId } = useGuildSelector();
  const { overrides, loaded, loading } = storeToRefs(store);
  const { role, isSuper } = storeToRefs(myRoleStore);

  watch(
    selectedGuildId,
    (gid) => {
      if (gid) void store.load(gid);
    },
    { immediate: true },
  );

  function visible(key: string): boolean {
    return store.visible(key);
  }

  async function reload(): Promise<void> {
    if (selectedGuildId.value) {
      store.invalidate();
      await store.load(selectedGuildId.value);
    }
  }

  return {
    visible,
    role,
    isSuper,
    loaded,
    loading,
    overrides,
    reload,
  };
}

/** Helper appele depuis useAppInit (router.beforeEach) pour prefetch. */
export async function preloadComponentVisibility(guildId: string): Promise<void> {
  await useComponentVisibilityStore().load(guildId);
}
