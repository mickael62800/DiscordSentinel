import { httpGet } from "@/api/http";
import type { Guild, GuildMember } from "@/types";

export interface DiscordTextChannel {
  id: string;
  name: string;
  position: number;
}

export const guildsService = {
  getAll(): Promise<Guild[]> { return httpGet("/api/guilds"); },
  getMembers(guildId: string): Promise<GuildMember[]> {
    return httpGet(`/api/guilds/${guildId}/members`);
  },
  getTextChannels(guildId: string): Promise<DiscordTextChannel[]> {
    return httpGet(`/api/guilds/${guildId}/channels`);
  },
};
