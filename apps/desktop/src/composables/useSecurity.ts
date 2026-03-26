import { useFetch } from "./useFetch";
import type { SecurityEvent } from "../types";

export function useSecurity() {
  const { data: events, loading, error, refresh: fetchEvents } = useFetch<SecurityEvent[]>(
    "get_security_events",
    [],
    { guildId: null },
  );

  return { events, loading, error, fetchEvents };
}
