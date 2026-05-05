import { storeToRefs } from "pinia";
import { watch } from "vue";
import { useBotEnabledStatusStore } from "@/stores/botEnabledStatusStore";
import { useGuildSelector } from "./useGuildSelector";

/**
 * Wrapper composable : delegue au store Pinia botEnabledStatus + integre
 * le watch sur guildIdFilter. API publique identique a la version
 * singleton precedente.
 */
export function useBotEnabledStatus() {
  const store = useBotEnabledStatusStore();
  const { guildIdFilter } = useGuildSelector();
  const { disabledBots, disabledCount, loading, error, enabledMap } = storeToRefs(store);

  watch(
    guildIdFilter,
    (gid) => {
      if (gid) void store.load(gid);
      else store.reset();
    },
    { immediate: true },
  );

  async function fetchConfigs(): Promise<void> {
    if (guildIdFilter.value) {
      store.invalidate();
      await store.load(guildIdFilter.value);
    }
  }

  return {
    // Garde la fct (utilisee ailleurs) ET expose enabledMap pour
    // permettre une utilisation directe en template — la fonction
    // wrapper casse le tracking de reactivite Vue dans certains cas
    // (Pinia setup store + closure).
    isBotEnabled: (name: string) => store.isBotEnabled(name),
    enabledMap,
    disabledBots,
    disabledCount,
    loading,
    error,
    fetchConfigs,
  };
}

/** Helper appele depuis useAppInit (router.beforeEach) pour prefetch. */
export async function preloadBotEnabledStatus(guildId: string): Promise<void> {
  await useBotEnabledStatusStore().load(guildId);
}
