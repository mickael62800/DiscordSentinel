import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { Infraction } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useInfractions() {
  const infractions = ref<Infraction[]>([]);
  const loading = ref(true);
  const error = ref<string | null>(null);
  const { guildIdFilter } = useGuildSelector();

  async function fetchInfractions() {
    loading.value = true;
    error.value = null;
    try {
      infractions.value = await invoke<Infraction[]>("get_infractions", { guildId: guildIdFilter.value ?? null });
    } catch (e) {
      error.value = String(e);
      console.error("Erreur chargement infractions:", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchInfractions);
  watch(guildIdFilter, fetchInfractions);

  return { infractions, loading, error, fetchInfractions };
}
