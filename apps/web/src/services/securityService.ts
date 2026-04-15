import { httpDelete, httpGet } from "@/api/http";
import type { SecurityEvent } from "@/types";
import { q } from "./_query";

export const securityService = {
  getEvents(guildId?: string | null): Promise<SecurityEvent[]> {
    return httpGet(`/api/security/events${q({ guild_id: guildId ?? null })}`);
  },
  purge(guildId: string): Promise<{ deleted_events: number; deleted_watches: number }> {
    return httpDelete(`/api/security/events/${guildId}`);
  },
};
