// Service web pour les sous-systèmes social/toxic du Coude (curses,
// bounties, coalitions, vendettas). Expose les endpoints existants côté API.

import { httpGet, httpPost } from "@/api/http";
import type {
  ActiveBounty,
  ActiveCoalition,
  ActiveCurse,
  ActiveVendetta,
} from "@/types/coude-social";

export const coudeSocialService = {
  // ── Curses (par target) ───────────────────────────────────
  getActiveCurse(guildId: string, targetId: string): Promise<ActiveCurse | null> {
    return httpGet(`/api/coude/${guildId}/curses/${targetId}`);
  },
  liftCurse(guildId: string, targetId: string, lifterId: string): Promise<unknown> {
    return httpPost(`/api/coude/${guildId}/curses/${targetId}/lift`, {
      lifter_id: lifterId,
    });
  },

  // ── Bounties (par target) ─────────────────────────────────
  getBountyByTarget(guildId: string, targetId: string): Promise<ActiveBounty | null> {
    return httpGet(`/api/coude/${guildId}/bounties/by-target/${targetId}`);
  },

  // ── Coalitions (par target) ───────────────────────────────
  getCoalitionByTarget(
    guildId: string,
    targetId: string,
  ): Promise<ActiveCoalition | null> {
    return httpGet(`/api/coude/${guildId}/coalitions/by-target/${targetId}`);
  },

  // ── Vendettas (par challenger) ────────────────────────────
  listVendettasByChallenger(
    guildId: string,
    challengerId: string,
  ): Promise<ActiveVendetta[]> {
    return httpGet(
      `/api/coude/${guildId}/vendettas/by-challenger/${challengerId}`,
    );
  },
};
