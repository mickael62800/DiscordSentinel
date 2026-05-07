import { ref, watch, type Ref } from "vue";
import type { FullAnalytics } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { analyticsService } from "@/services/analyticsService";

/**
 * Si `externalDays` est fourni, utilise ce ref comme source unique
 * (partage avec d'autres sections). Sinon, ref interne.
 */
export function useAnalytics(externalDays?: Ref<number>) {
  const { error: showError } = useToast();
  const analytics = ref<FullAnalytics | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const days = externalDays ?? ref(30);
  const { guildIdFilter } = useGuildSelector();

  async function fetchAnalytics() {
    loading.value = true;
    error.value = null;
    try {
      analytics.value = await analyticsService.getFull(guildIdFilter.value ?? null, days.value);
    } catch (e) {
      error.value = String(e);
      console.error("Erreur chargement analytics :", e);
      showError("Erreur lors du chargement des analytics.");
    } finally {
      loading.value = false;
    }
  }

  watch([guildIdFilter, days], fetchAnalytics, { immediate: true });

  return { analytics, loading, error, days, fetchAnalytics };
}
