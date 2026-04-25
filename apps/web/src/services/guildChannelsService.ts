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
};
