import { httpGet } from "@/api/http";
import type { Guild, GuildMember } from "@/types";

export const guildsService = {
  getAll(): Promise<Guild[]> { return httpGet("/api/guilds"); },
  getMembers(guildId: string): Promise<GuildMember[]> {
    return httpGet(`/api/guilds/${guildId}/members`);
  },
};
