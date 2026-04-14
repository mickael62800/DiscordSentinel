import { httpGet } from "@/api/http";
import type { RolePanel, RolePanelDetail, AutoRoleConfig } from "@/types";

export const rolePanelsService = {
  getAll(guildId: string): Promise<RolePanel[]> {
    return httpGet(`/api/role-panels/${guildId}`);
  },
  getDetail(panelId: string): Promise<RolePanelDetail> {
    return httpGet(`/api/role-panels/detail/${panelId}`);
  },
  getAutoRoles(guildId: string): Promise<AutoRoleConfig[]> {
    return httpGet(`/api/auto-roles/${guildId}`);
  },
};
