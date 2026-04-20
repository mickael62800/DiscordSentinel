import { httpGet, httpDelete, httpPatch, httpPut } from "@/api/http";
import type { CoudeCombat, CoudePlayer } from "@/types";
import { q } from "./_query";

// Phase 9 Part E — config railleries
export interface TauntsConfig {
  guild_id: string;
  channel_id: string | null;
  enabled: boolean;
  rename_enabled: boolean;
  messages_enabled: boolean;
  opt_outs: string[];
}

export interface UpdateTauntsConfigPayload {
  channel_id: string | null;
  enabled: boolean;
  rename_enabled?: boolean;
  messages_enabled?: boolean;
}

export const coudeService = {
  getCombats(guildId: string, status?: string | null): Promise<CoudeCombat[]> {
    return httpGet(`/api/coude/${guildId}/combats${q({ status })}`);
  },
  getPlayers(guildId: string): Promise<CoudePlayer[]> {
    return httpGet(`/api/coude/${guildId}/players`);
  },
  cancelCombat(combatId: string): Promise<void> {
    return httpDelete(`/api/coude/combats/${combatId}`);
  },
  adjustCoins(guildId: string, userId: string, amount: number): Promise<unknown> {
    return httpPatch(`/api/coude/players/${guildId}/${userId}/coins`, { amount });
  },
  purgeAll(guildId: string): Promise<Record<string, number>> {
    return httpDelete(`/api/coude/${guildId}/purge`);
  },

  // ── Railleries (Phase 9 Part D/E) ──

  getTauntsConfig(guildId: string): Promise<TauntsConfig> {
    return httpGet(`/api/coude/${guildId}/config/taunts`);
  },
  updateTauntsConfig(
    guildId: string,
    payload: UpdateTauntsConfigPayload,
  ): Promise<void> {
    return httpPut(`/api/coude/${guildId}/config/taunts`, payload);
  },
  removeTauntOptOut(guildId: string, userId: string): Promise<void> {
    return httpDelete(`/api/coude/${guildId}/config/taunts/opt-outs/${userId}`);
  },
};
