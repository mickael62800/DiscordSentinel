import { computed, onMounted, ref, watch } from "vue";
import { botConfigService } from "@/services/botConfigService";
import { useGuildSelector } from "./useGuildSelector";
import type { BotGuildConfig } from "@/types";

/**
 * Etat enabled/disabled des bots pour la guild selectionnee.
 *
 * Convention : un bot est ENABLED par defaut. Il n'est DISABLED que si une
 * row existe avec `config_key = "enabled"` ET `config_value = "false"`.
 * (Match la logique cote bot, fail-open : pas de row → on assume actif.)
 *
 * Utilise par la dashboard pour cacher les boutons dont le composant est OFF.
 * Si pas de guild selectionnee → tous les bots consideres comme actifs (vue
 * "global" sur tous serveurs n'a pas de notion de bot disabled par guild).
 */
export function useBotEnabledStatus() {
  const { guildIdFilter } = useGuildSelector();
  const configs = ref<BotGuildConfig[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function fetchConfigs() {
    if (!guildIdFilter.value) {
      configs.value = [];
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      configs.value = await botConfigService.getGuildConfig(guildIdFilter.value);
    } catch (e) {
      error.value = String(e);
      configs.value = [];
    } finally {
      loading.value = false;
    }
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

  /** True si le bot est actif sur la guild courante (default = true).
   *  Si pas de guild selectionnee → true (mode global). */
  function isBotEnabled(botName: string): boolean {
    if (!guildIdFilter.value) return true;
    const v = enabledMap.value[botName];
    return v === undefined ? true : v;
  }

  /** Liste des bots explicitement desactives (config_value = "false"). */
  const disabledBots = computed<string[]>(() => {
    return Object.entries(enabledMap.value)
      .filter(([, enabled]) => !enabled)
      .map(([name]) => name);
  });

  /** Nombre de composants desactives sur la guild courante. */
  const disabledCount = computed(() => disabledBots.value.length);

  onMounted(fetchConfigs);
  watch(guildIdFilter, fetchConfigs);

  return { isBotEnabled, disabledBots, disabledCount, loading, error, fetchConfigs };
}
