import type { SecurityEvent } from "../types";
import { useGuildFetch } from "./useGuildFetch";

export function useSecurity() {
  const { data: events, loading, error, refresh: fetchEvents } = useGuildFetch<SecurityEvent[]>(
    "get_security_events",
    [],
  );

  return { events, loading, error, fetchEvents };
}
