import { httpGet, httpPost, httpPatch, httpDelete, httpPut } from "@/api/http";
import type { ComponentVisibilityEntry, GuildUserRole, MyRole, RbacRole } from "@/types";

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
  listComponentVisibility(guildId: string): Promise<ComponentVisibilityEntry[]> {
    return httpGet(`/api/rbac/component-visibility/${guildId}`);
  },
  upsertComponentVisibility(guildId: string, entries: ComponentVisibilityEntry[]): Promise<unknown> {
    return httpPut(`/api/rbac/component-visibility/${guildId}`, { entries });
  },

  // Gates RBAC granulaires (purges, resets — ce que peut faire chaque role).
  listComponentMinRoles(guildId: string): Promise<ComponentMinRoleEntry[]> {
    return httpGet(`/api/rbac/component-min-role/${guildId}`);
  },
  upsertComponentMinRole(
    guildId: string,
    componentKey: string,
    minRole: RbacRole,
  ): Promise<unknown> {
    return httpPut(`/api/rbac/component-min-role/${guildId}`, {
      component_key: componentKey,
      min_role: minRole,
    });
  },
  deleteComponentMinRole(guildId: string, componentKey: string): Promise<void> {
    return httpDelete(`/api/rbac/component-min-role/${guildId}/${componentKey}`);
  },
};

export interface ComponentMinRoleEntry {
  component_key: string;
  default_role: RbacRole;
  floor_role: RbacRole;
  /** Role effectif applique (override si present, sinon default). */
  effective_role: RbacRole;
  /** Override explicite stocke en DB, null si default. */
  override_role: RbacRole | null;
}
