import type { SecurityEvent } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { securityService } from "@/services/securityService";

export function useSecurity() {
  const { data: events, loading, error, refresh: fetchEvents } = useGuildFetch<SecurityEvent[]>(
    (guildId) => securityService.getEvents(guildId),
    [],
    { label: "evenements de securite" },
  );

  return { events, loading, error, fetchEvents };
}
