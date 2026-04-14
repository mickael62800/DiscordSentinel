import { httpGet, httpDelete, httpPatch } from "@/api/http";
import type { CoudeCombat, CoudePlayer } from "@/types";
import { q } from "./_query";

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
};
