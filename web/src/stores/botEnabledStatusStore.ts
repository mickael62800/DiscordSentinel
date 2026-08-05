import { defineStore } from "pinia";
import { computed, ref } from "vue";
import type { BotGuildConfig } from "@/types";
import { botConfigService } from "@/services/botConfigService";
import { parseBoolConfig } from "@/utils/configFlags";

/**
 * Store Pinia : etat enabled/disabled des bots pour la guild courante.
 *
 * Convention : un bot est DISABLED par defaut. Il n'est ENABLED que si une
 * row existe avec config_key="enabled" et une valeur vraie. Miroir exact de
 * `parse_enabled_flag` cote Rust (fail-closed) : le dashboard doit montrer
 * ce que le bot fait reellement, pas l'inverse.
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
        map[c.bot_name] = parseBoolConfig(c.config_value);
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
   * True si le bot est actif sur la guild courante (default = false).
   * Tant que les configs ne sont pas chargees, on ne prejuge de rien et on
   * repond false : mieux vaut afficher un module inactif une fraction de
   * seconde que promettre un module actif qui ne l'est pas.
   */
  function isBotEnabled(botName: string): boolean {
    return enabledMap.value[botName] === true;
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
