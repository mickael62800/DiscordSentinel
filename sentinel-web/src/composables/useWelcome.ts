import { ref, watch } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { welcomeService } from "@/services/welcomeService";
import type { WelcomeConfig, SaveWelcomeConfigParams } from "@/types/welcome";

// State module-scoped : un seul cache partage entre la page et le form
// organism. Sans ca, chaque appel useWelcome() creerait son propre state
// et le form ne verrait pas le config charge par la page.
const { guildIdFilter } = useGuildSelector();

const config = ref<WelcomeConfig | null>(null);
const loading = ref(true);
const saving = ref(false);

async function fetchConfig() {
  const { error: showError } = useToast();
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

// Auto-fetch au demarrage et au changement de guild (immediate au 1er import).
watch(guildIdFilter, fetchConfig, { immediate: true });

export function useWelcome() {
  const { success, error: showError } = useToast();

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

  return { config, loading, saving, fetchConfig, saveConfig };
}
