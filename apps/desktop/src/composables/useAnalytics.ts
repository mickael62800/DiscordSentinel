import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { FullAnalytics } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useAnalytics() {
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
      console.error("Erreur chargement analytics:", e);
    } finally {
      loading.value = false;
    }
  }

  watch([guildIdFilter, days], fetchAnalytics, { immediate: true });

  return { analytics, loading, error, days, fetchAnalytics };
}
