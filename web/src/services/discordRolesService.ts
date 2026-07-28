import { httpGet, httpPost, httpPatch, httpDelete } from "@/api/http";
import type { DiscordRole } from "@/types";

export interface CreateRoleParams {
  name: string;
  color: number;
  permissions?: string | null;
}

export interface EditRoleParams {
  name?: string | null;
  color?: number;
  permissions?: string;
  mentionable?: boolean;
  hoist?: boolean;
}

export const discordRolesService = {
  getAll(guildId: string): Promise<DiscordRole[]> {
    return httpGet(`/api/discord-roles/${guildId}`);
  },
  create(guildId: string, params: CreateRoleParams): Promise<unknown> {
    return httpPost(`/api/discord-roles/${guildId}/create`, params);
  },
  edit(guildId: string, roleId: string, params: EditRoleParams): Promise<unknown> {
    return httpPatch(`/api/discord-roles/${guildId}/${roleId}`, params);
  },
  remove(guildId: string, roleId: string): Promise<unknown> {
    return httpDelete(`/api/discord-roles/${guildId}/${roleId}`);
  },
};
