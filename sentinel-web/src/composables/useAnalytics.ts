import { ref, type Ref } from "vue";
import type { FullAnalytics } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { analyticsService } from "@/services/analyticsService";

/**
 * Si `externalDays` est fourni, utilise ce ref comme source unique
 * (partage avec d'autres sections). Sinon, ref interne.
 */
export function useAnalytics(externalDays?: Ref<number>) {
  const days = externalDays ?? ref(30);

  const { data: analytics, loading, error, refresh: fetchAnalytics } = useGuildFetch<FullAnalytics | null>(
    (guildId) => analyticsService.getFull(guildId, days.value),
    null,
    { label: "analytics", watchSources: [days] },
  );

  return { analytics, loading, error, days, fetchAnalytics };
}
