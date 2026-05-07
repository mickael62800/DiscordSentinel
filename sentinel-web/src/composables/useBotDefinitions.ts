import { storeToRefs } from "pinia";
import { useBotDefinitionsStore } from "@/stores/botDefinitionsStore";

/**
 * Wrapper composable : delegue au store Pinia botDefinitions.
 * API publique identique a la version singleton precedente.
 */
export function useBotDefinitions() {
  const store = useBotDefinitionsStore();
  const { definitions, loaded, loading } = storeToRefs(store);

  // Trigger load au 1er appel si pas encore fait.
  if (!loaded.value) {
    void store.ensure();
  }

  return {
    definitions,
    loaded,
    loading,
    ensure: () => store.ensure(),
  };
}

/** Helper appele depuis useAppInit (router.beforeEach) pour prefetch. */
export async function preloadBotDefinitions(): Promise<void> {
  await useBotDefinitionsStore().ensure();
}
