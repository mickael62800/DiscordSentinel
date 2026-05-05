import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { BotGuildConfig } from "@/types";
import { botConfigService } from "@/services/botConfigService";

/**
 * Store Pinia : etat enabled/disabled des bots pour la guild courante.
 *
 * Convention : un bot est ENABLED par defaut. Il n'est DISABLED que si
 * une row existe avec config_key="enabled" ET config_value="false".
 * (Match la logique cote bot, fail-open.)
 *
 * Visible dans Vue DevTools sous "botEnabledStatus".
 */
export const useBotEnabledStatusStore = defineStore("botEnabledStatus", () => {
  const configs = ref<BotGuildConfig[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  let lastLoadedGuild: string | null = null;
  let inFlight: Promise<void> | null = null;

  /** Map<bot_name, enabled> derivee des configs. */
  const enabledMap = computed<Record<string, boolean>>(() => {
    const map: Record<string, boolean> = {};
    for (const c of configs.value) {
      if (c.config_key === "enabled") {
        map[c.bot_name] = c.config_value === "true" || c.config_value === "1";
      }
    }
    return map;
  });

  const disabledBots = computed<string[]>(() =>
    Object.entries(enabledMap.value)
      .filter(([, enabled]) => !enabled)
      .map(([name]) => name),
  );

  const disabledCount = computed(() => disabledBots.value.length);

  async function load(guildId: string): Promise<void> {
    // Deja charge pour cette guild ET aucun load en cours : rien a faire.
    if (lastLoadedGuild === guildId && !inFlight) return;
    // Un load est en cours : on attend son resultat (peu importe la guild :
    // au boot, plusieurs composants appellent load() en parallele AVANT que
    // lastLoadedGuild soit set, et on doit dedupliquer pour eviter 4-5 GET).
    if (inFlight) return inFlight;

    loading.value = true;
    error.value = null;
    inFlight = (async () => {
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

  function reset(): void {
    configs.value = [];
    lastLoadedGuild = null;
  }

  function invalidate(): void {
    lastLoadedGuild = null;
  }

  /**
   * True si le bot est actif sur la guild courante (default = true).
   * Si pas de guild selectionnee → true (mode global).
   */
  function isBotEnabled(botName: string): boolean {
    if (configs.value.length === 0) return true;
    const v = enabledMap.value[botName];
    return v === undefined ? true : v;
  }

  return {
    configs,
    enabledMap,
    disabledBots,
    disabledCount,
    loading,
    error,
    load,
    reset,
    invalidate,
    isBotEnabled,
  };
});
