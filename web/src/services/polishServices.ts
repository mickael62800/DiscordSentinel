// Services pour Phase 10 — sponsorships, temp-roles, system.

import { httpDelete, httpGet, httpPost } from "@/api/http";
import type {
  CacheStats,
  CreateSponsorshipPayload,
  CreateTempRolePayload,
  ModelsStatusResponse,
  Sponsorship,
  TempRole,
} from "@/types/polish";

export const sponsorshipsService = {
  list(guildId: string): Promise<Sponsorship[]> {
    return httpGet(`/api/sponsorships/${guildId}`);
  },
  create(body: CreateSponsorshipPayload): Promise<Sponsorship> {
    return httpPost(`/api/sponsorships`, body);
  },
};

export const tempRolesService = {
  list(guildId: string): Promise<TempRole[]> {
    return httpGet(`/api/temp-roles/${guildId}`);
  },
  create(body: CreateTempRolePayload): Promise<TempRole> {
    return httpPost(`/api/temp-roles`, body);
  },
  remove(guildId: string, userId: string, roleId: string): Promise<unknown> {
    return httpDelete(`/api/temp-roles/${guildId}/${userId}/${roleId}`);
  },
};

export const systemOpsService = {
  getModelsStatus(): Promise<ModelsStatusResponse> {
    return httpGet(`/api/models/status`);
  },
  reloadModel(modelType: string): Promise<unknown> {
    return httpPost(`/api/models/reload`, { model_type: modelType });
  },
  getCacheStats(): Promise<CacheStats> {
    return httpGet(`/api/cache/stats`);
  },
};
