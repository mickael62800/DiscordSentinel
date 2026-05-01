import { defineStore } from "pinia";
import { ref } from "vue";
import type { BotDefinition } from "@/types";
import { botConfigService } from "@/services/botConfigService";

/**
 * Store Pinia : definitions des bots (donnees stables, rarement changent).
 *
 * Charge une seule fois par session via `ensure()`. Backend a un cache
 * Redis 1h, et on ajoute notre cache memoire pour ne pas le solliciter
 * a chaque navigation entre pages.
 *
 * Visible dans Vue DevTools sous "botDefinitions".
 */
export const useBotDefinitionsStore = defineStore("botDefinitions", () => {
  const definitions = ref<BotDefinition[]>([]);
  const loaded = ref(false);
  const loading = ref(false);

  let inFlight: Promise<BotDefinition[]> | null = null;

  async function ensure(): Promise<BotDefinition[]> {
    if (loaded.value) return definitions.value;
    if (inFlight) return inFlight;

    loading.value = true;
    inFlight = (async () => {
      try {
        const list = await botConfigService.getDefinitions();
        definitions.value = list;
        loaded.value = true;
        return list;
      } finally {
        loading.value = false;
        inFlight = null;
      }
    })();
    return inFlight;
  }

  function invalidate(): void {
    loaded.value = false;
  }

  return { definitions, loaded, loading, ensure, invalidate };
});
