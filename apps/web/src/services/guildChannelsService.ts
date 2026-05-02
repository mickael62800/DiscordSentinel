import { httpGet } from "@/api/http";
import type { DiscordChannelInfo } from "@/types";

export const guildChannelsService = {
  /**
   * Liste les salons texte d'une guild Discord.
   * L'API met les resultats en cache Redis 10 min.
   */
  listTextChannels(guildId: string): Promise<DiscordChannelInfo[]> {
    return httpGet(`/api/guilds/${guildId}/channels`);
  },
  /**
   * Liste tous les salons (texte + voice + stage) avec un champ `kind`.
   * Utilise par les pickers config qui s'appliquent aux deux types
   * (xp_channel_multipliers, etc.).
   */
  listAllChannels(guildId: string): Promise<DiscordChannelInfo[]> {
    return httpGet(`/api/guilds/${guildId}/channels/all`);
  },
};
