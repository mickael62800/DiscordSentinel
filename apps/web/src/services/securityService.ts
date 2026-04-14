import { httpGet } from "@/api/http";
import type { SecurityEvent } from "@/types";
import { q } from "./_query";

export const securityService = {
  getEvents(guildId?: string | null): Promise<SecurityEvent[]> {
    return httpGet(`/api/security/events${q({ guild_id: guildId ?? null })}`);
  },
};
