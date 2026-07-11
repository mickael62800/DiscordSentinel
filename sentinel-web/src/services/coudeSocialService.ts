// Service web pour les sous-systèmes social/toxic du Coude réellement exposés
// par l'API : curses (malédictions) et primes (bounties).

import { httpGet, httpPost } from "@/api/http";
import type { ActiveCurse, Prime } from "@/types/coude-social";

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

  // ── Primes / bounties (par target) ────────────────────────
  // Liste des primes actives posées sur la cible.
  listActivePrimes(guildId: string, targetId: string): Promise<Prime[]> {
    return httpGet(`/api/coude/${guildId}/primes/${targetId}/active`);
  },
};
