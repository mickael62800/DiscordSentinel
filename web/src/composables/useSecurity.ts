import type { SecurityEvent } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { securityService } from "@/services/securityService";

// Singleton module-scoped : un seul cache partage entre StatsGrid et EventsList.
const { data: events, loading, error, refresh: fetchEvents } = useGuildFetch<SecurityEvent[]>(
  (guildId) => securityService.getEvents(guildId),
  [],
  { label: "evenements de securite" },
);

export function useSecurity() {
  return { events, loading, error, fetchEvents };
}
