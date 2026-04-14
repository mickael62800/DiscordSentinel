import { httpGet, httpDelete } from "@/api/http";
import type { Infraction } from "@/types";
import { q } from "./_query";

export const infractionsService = {
  getAll(guildId?: string | null): Promise<Infraction[]> {
    return httpGet(`/api/infractions${q({ guild_id: guildId ?? null })}`);
  },
  remove(id: string): Promise<void> {
    return httpDelete(`/api/infractions/${id}`);
  },
  /**
   * Supprime TOUTES les infractions d'une guild.
   * Utilise `/api/purge/infractions` avec `days: 0` (pas de filtre de date).
   * Necessite le role Owner sur la guild ou d'etre superadmin.
   */
  purgeAll(guildId: string): Promise<{ deleted: number }> {
    return httpDelete(`/api/purge/infractions`, { guild_id: guildId, days: 0 });
  },
};
