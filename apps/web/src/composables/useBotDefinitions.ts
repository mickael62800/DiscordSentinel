import { ref } from "vue";
import { botConfigService } from "@/services/botConfigService";
import type { BotDefinition } from "@/types";

/**
 * Singleton bot definitions — donnees stables (rarement changent).
 *
 * Avant : chaque page (ComponentConfigPage, useBotEnabledStatus indirect, etc.)
 * appelait /api/bots/definitions independamment a son onMounted -> 3-4 requetes
 * pour le meme JSON identique.
 *
 * Maintenant : une seule requete partagee en module-scope. Backend a deja un
 * cache Redis 1h, mais autant ne pas le solliciter inutilement.
 */

const definitions = ref<BotDefinition[]>([]);
const loaded = ref(false);
const loading = ref(false);
let inFlight: Promise<BotDefinition[]> | null = null;

async function load(): Promise<BotDefinition[]> {
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

export async function preloadBotDefinitions(): Promise<void> {
  await load();
}

export function useBotDefinitions() {
  // Trigger load si pas encore fait. Composants peuvent attendre via `loaded`
  // ou directement appeler `await load()` via `ensure()`.
  if (!loaded.value && !inFlight) {
    void load();
  }

  return {
    definitions,
    loaded,
    loading,
    /** Force la chargement et attend le resultat. */
    ensure: load,
  };
}
