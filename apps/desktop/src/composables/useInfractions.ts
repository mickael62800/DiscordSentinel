import type { Infraction } from "../types";
import { useGuildFetch } from "./useGuildFetch";

export function useInfractions() {
  const { data: infractions, loading, error, refresh: fetchInfractions } = useGuildFetch<Infraction[]>(
    "get_infractions",
    [],
  );

  return { infractions, loading, error, fetchInfractions };
}
