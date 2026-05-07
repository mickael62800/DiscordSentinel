import { httpDelete, httpGet, httpPost } from "@/api/http";
import type { RolePanel, RolePanelDetail, AutoRoleConfig } from "@/types";

export interface CreateRolePanelEntryPayload {
  role_id: string;
  role_name: string;
  emoji?: string | null;
  label?: string;
  style?: string; // "primary" | "secondary" | "success" | "danger"
  position?: number;
}

export interface CreateRolePanelPayload {
  guild_id: string;
  channel_id: string;
  title: string;
  description?: string;
  mode?: string; // "button" | "select"
  max_roles?: number | null;
  entries: CreateRolePanelEntryPayload[];
}

export interface CreateAutoRolePayload {
  guild_id: string;
  role_id: string;
  role_name: string;
  delay_secs?: number;
}

export const rolePanelsService = {
  getAll(guildId: string): Promise<RolePanel[]> {
    return httpGet(`/api/role-panels/${guildId}`);
  },
  getDetail(panelId: string): Promise<RolePanelDetail> {
    return httpGet(`/api/role-panels/detail/${panelId}`);
  },
  create(body: CreateRolePanelPayload): Promise<RolePanelDetail> {
    return httpPost(`/api/role-panels`, body);
  },
  remove(panelId: string): Promise<unknown> {
    return httpDelete(`/api/role-panels/detail/${panelId}`);
  },
  getAutoRoles(guildId: string): Promise<AutoRoleConfig[]> {
    return httpGet(`/api/auto-roles/${guildId}`);
  },
  addAutoRole(body: CreateAutoRolePayload): Promise<AutoRoleConfig> {
    return httpPost(`/api/auto-roles`, body);
  },
  removeAutoRole(guildId: string, roleId: string): Promise<unknown> {
    return httpDelete(`/api/auto-roles/${guildId}/${roleId}`);
  },
};
