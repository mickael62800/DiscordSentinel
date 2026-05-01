import { computed, ref, watch } from "vue";
import { botConfigService } from "@/services/botConfigService";
import { useGuildSelector } from "./useGuildSelector";
import type { BotGuildConfig } from "@/types";

/**
 * Etat enabled/disabled des bots pour la guild selectionnee.
 *
 * Convention : un bot est ENABLED par defaut. Il n'est DISABLED que si une
 * row existe avec `config_key = "enabled"` ET `config_value = "false"`.
 *
 * SINGLETON : etat partage en module-scope -> une seule requete par changement
 * de guild, peu importe combien de composants appellent useBotEnabledStatus().
 */

// Etat partage (module-scope)
const configs = ref<BotGuildConfig[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
let lastLoadedGuild: string | null = null;
let inFlight: Promise<void> | null = null;

async function loadFor(guildId: string): Promise<void> {
  // Evite les requetes paralleles pour la meme guild
  if (inFlight && lastLoadedGuild === guildId) return inFlight;
  if (lastLoadedGuild === guildId) return;

  inFlight = (async () => {
    loading.value = true;
    error.value = null;
    try {
      configs.value = await botConfigService.getGuildConfig(guildId);
      lastLoadedGuild = guildId;
    } catch (e) {
      error.value = String(e);
      configs.value = [];
    } finally {
      loading.value = false;
      inFlight = null;
    }
  })();
  return inFlight;
}

export async function preloadBotEnabledStatus(guildId: string): Promise<void> {
  await loadFor(guildId);
}

export function useBotEnabledStatus() {
  const { guildIdFilter } = useGuildSelector();

  async function fetchConfigs() {
    if (!guildIdFilter.value) {
      configs.value = [];
      lastLoadedGuild = null;
      return;
    }
    await loadFor(guildIdFilter.value);
  }

  /** Map<bot_name, enabled> calcule une seule fois par changement de configs. */
  const enabledMap = computed<Record<string, boolean>>(() => {
    const map: Record<string, boolean> = {};
    for (const c of configs.value) {
      if (c.config_key === "enabled") {
        map[c.bot_name] = c.config_value === "true" || c.config_value === "1";
      }
    }
    return map;
  });

  /** True si le bot est actif sur la guild courante (default = true). */
  function isBotEnabled(botName: string): boolean {
    if (!guildIdFilter.value) return true;
    const v = enabledMap.value[botName];
    return v === undefined ? true : v;
  }

  const disabledBots = computed<string[]>(() => {
    return Object.entries(enabledMap.value)
      .filter(([, enabled]) => !enabled)
      .map(([name]) => name);
  });

  const disabledCount = computed(() => disabledBots.value.length);

  // Charge si pas encore charge pour la guild courante
  if (guildIdFilter.value && lastLoadedGuild !== guildIdFilter.value) {
    void fetchConfigs();
  }
  watch(guildIdFilter, () => {
    lastLoadedGuild = null;
    void fetchConfigs();
  });

  return { isBotEnabled, disabledBots, disabledCount, loading, error, fetchConfigs };
}
