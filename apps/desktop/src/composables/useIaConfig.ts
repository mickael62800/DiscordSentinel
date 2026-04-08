import { ref, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { IaConfig, SaveIaConfigParams } from "../types";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

export function useIaConfig() {
  const { success, error: showError } = useToast();
  const config = ref<IaConfig | null>(null);
  const loading = ref(false);
  const saving = ref(false);
  const error = ref<string | null>(null);
  const { guildIdFilter } = useGuildSelector();

  async function fetchConfig() {
    const guildId = guildIdFilter.value;
    if (!guildId) {
      config.value = null;
      return;
    }

    loading.value = true;
    error.value = null;
    try {
      config.value = await invoke<IaConfig>("get_ia_config", { guildId });
    } catch (e) {
      error.value = String(e);
      console.error("Erreur lors du chargement de la configuration IA :", e);
      showError("Erreur lors du chargement de la configuration IA.");
    } finally {
      loading.value = false;
    }
  }

  async function saveConfig(params: SaveIaConfigParams) {
    const guildId = guildIdFilter.value;
    if (!guildId) return;

    saving.value = true;
    error.value = null;
    try {
      config.value = await invoke<IaConfig>("save_ia_config", {
        guildId,
        textEnabled: params.text_enabled,
        textThreshold: params.text_threshold,
        visionEnabled: params.vision_enabled,
        visionThreshold: params.vision_threshold,
      });
      success("Configuration IA sauvegardee avec succes.");
    } catch (e) {
      error.value = String(e);
      console.error("Erreur lors de la sauvegarde de la configuration IA :", e);
      showError("Erreur lors de la sauvegarde de la configuration IA.");
    } finally {
      saving.value = false;
    }
  }

  watch(guildIdFilter, fetchConfig, { immediate: true });

  return { config, loading, saving, error, fetchConfig, saveConfig };
}
