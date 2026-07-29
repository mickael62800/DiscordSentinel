// Configuration des modules de la plateforme jeux Nexus.
//
// Meme contrat que `botConfigService` (cote Sentinel) pour que le formulaire
// generique `ComponentConfigForm` fonctionne a l'identique, mais l'API Nexus
// expose la config a plat (`{ cle: valeur }`) et non sous forme de lignes.
// La conversion se fait ici.

import { nexusGet, nexusPut } from "@/api/nexusHttp";
import type { BotDefinition, BotGuildConfig } from "@/types";

export const nexusConfigService = {
  /** GET /api/bots/definitions — modules Nexus et leur schema. */
  getDefinitions(guildId: string): Promise<BotDefinition[]> {
    return nexusGet<BotDefinition[]>("/api/bots/definitions", guildId);
  },

  /**
   * GET /api/config/{guild}/{bot} — config d'un module, remise au format
   * ligne par ligne attendu par le formulaire.
   */
  async getGuildConfig(guildId: string, botName: string): Promise<BotGuildConfig[]> {
    const flat = await nexusGet<Record<string, string>>(
      `/api/config/${encodeURIComponent(guildId)}/${encodeURIComponent(botName)}`,
      guildId,
    );
    return Object.entries(flat ?? {}).map(([config_key, config_value]) => ({
      guild_id: guildId,
      bot_name: botName,
      config_key,
      config_value,
    }));
  },

  /** PUT /api/config/{guild}/{bot} — enregistre une cle. */
  set(guildId: string, botName: string, key: string, value: string): Promise<unknown> {
    return nexusPut(
      `/api/config/${encodeURIComponent(guildId)}/${encodeURIComponent(botName)}`,
      guildId,
      { key, value },
    );
  },

  /**
   * Nexus n'expose pas de suppression : une valeur vide equivaut a « non
   * configure » cote bot (les defauts du schema prennent le relais).
   */
  remove(guildId: string, botName: string, key: string): Promise<unknown> {
    return this.set(guildId, botName, key, "");
  },
};
