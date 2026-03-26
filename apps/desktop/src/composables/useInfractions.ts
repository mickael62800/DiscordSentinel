import { useFetch } from "./useFetch";
import type { Infraction } from "../types";

export function useInfractions() {
  const { data: infractions, loading, error, refresh: fetchInfractions } = useFetch<Infraction[]>(
    "get_infractions",
    [],
  );

  return { infractions, loading, error, fetchInfractions };
}
