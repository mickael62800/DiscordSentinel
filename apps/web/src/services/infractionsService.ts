import { httpGet, httpDelete } from "@/api/http";
import type { Infraction } from "@/types";
import { q } from "./_query";

export const infractionsService = {
  getAll(guildId?: string | null): Promise<Infraction[]> {
    return httpGet(`/api/infractions${q({ guild_id: guildId ?? null })}`);
  },
  /**
   * Supprime une ligne du journal. Selon la source :
   * - "detection" → DELETE /api/infractions/{id} (table infractions, automod)
   * - "action"    → DELETE /api/moderation/actions/{id} (table moderation_actions,
   *                 applique aussi un unban Discord si c'etait un ban)
   */
  remove(id: string, source: "detection" | "action" = "detection"): Promise<void> {
    if (source === "action") {
      return httpDelete(`/api/moderation/actions/${id}`);
    }
    return httpDelete(`/api/infractions/${id}`);
  },
  /**
   * Supprime TOUTES les infractions d'une guild.
   * Utilise `/api/purge/infractions` avec `days: 0` (pas de filtre de date).
   * Necessite le role Owner sur la guild ou d'etre superadmin.
   */
  purgeAll(guildId: string): Promise<{ deleted: number; points_restored: number }> {
    return httpDelete(`/api/purge/infractions`, { guild_id: guildId, days: 0 });
  },
};
