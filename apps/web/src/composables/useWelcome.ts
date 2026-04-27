import { ref, watch, onMounted } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { welcomeService } from "@/services/welcomeService";
import type { WelcomeConfig, SaveWelcomeConfigParams } from "@/types/welcome";

export function useWelcome() {
  const { guildIdFilter } = useGuildSelector();
  const { success, error: showError } = useToast();

  const config = ref<WelcomeConfig | null>(null);
  const loading = ref(true);
  const saving = ref(false);

  async function fetchConfig() {
    if (!guildIdFilter.value) {
      config.value = null;
      loading.value = false;
      return;
    }
    loading.value = true;
    try {
      config.value = await welcomeService.getConfig(guildIdFilter.value);
    } catch (e) {
      console.error("Erreur chargement config welcome :", e);
      showError("Impossible de charger la configuration de bienvenue.");
    } finally {
      loading.value = false;
    }
  }

  async function saveConfig(patch: SaveWelcomeConfigParams) {
    if (!guildIdFilter.value) return;
    saving.value = true;
    try {
      config.value = await welcomeService.saveConfig(guildIdFilter.value, patch);
      success("Configuration enregistree.");
    } catch (e) {
      console.error("Erreur sauvegarde config welcome :", e);
      showError("Erreur lors de la sauvegarde.");
      throw e;
    } finally {
      saving.value = false;
    }
  }

  onMounted(fetchConfig);
  watch(guildIdFilter, fetchConfig);

  return { config, loading, saving, fetchConfig, saveConfig };
}
