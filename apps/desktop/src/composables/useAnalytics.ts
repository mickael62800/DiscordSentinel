import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { FullAnalytics } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useRealtimeRefresh } from "./useRealtimeRefresh";
import { useToast } from "./useToast";

export function useAnalytics() {
  const { error: showError } = useToast();
  const analytics = ref<FullAnalytics | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const days = ref(30);
  const { guildIdFilter } = useGuildSelector();

  async function fetchAnalytics() {
    loading.value = true;
    error.value = null;
    try {
      analytics.value = await invoke<FullAnalytics>("get_full_analytics", {
        guildId: guildIdFilter.value ?? null,
        days: days.value,
      });
    } catch (e) {
      error.value = String(e);
      console.error("Erreur chargement analytics :", e);
      showError("Erreur lors du chargement des analytics.");
    } finally {
      loading.value = false;
    }
  }

  watch([guildIdFilter, days], fetchAnalytics, { immediate: true });

  // Refresh automatique quand des infractions/modérations arrivent
  useRealtimeRefresh(
    ["infraction_new", "moderation_action"],
    fetchAnalytics,
    { debounceMs: 10000 }, // 10s — analytics est une requete lourde
  );

  return { analytics, loading, error, days, fetchAnalytics };
}
