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
};
