import { httpGet, httpPost, httpPatch, httpDelete } from "@/api/http";
import type { GuildUserRole, MyRole, RbacRole } from "@/types";

export const rbacService = {
  listGuildUsers(guildId: string): Promise<GuildUserRole[]> {
    return httpGet(`/api/rbac/guilds/${guildId}/users`);
  },
  getMyRole(guildId: string): Promise<MyRole> {
    return httpGet(`/api/rbac/me/${guildId}`);
  },
  grantRole(guildId: string, userId: string, role: RbacRole, displayName?: string | null): Promise<unknown> {
    return httpPost(`/api/rbac/guilds/${guildId}/users/${userId}`, { role, display_name: displayName ?? null });
  },
  updateRole(guildId: string, userId: string, role: RbacRole): Promise<unknown> {
    return httpPatch(`/api/rbac/guilds/${guildId}/users/${userId}`, { role });
  },
  revokeRole(guildId: string, userId: string): Promise<void> {
    return httpDelete(`/api/rbac/guilds/${guildId}/users/${userId}`);
  },
};
