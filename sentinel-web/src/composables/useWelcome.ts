import { ref } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { useGuildFetch } from "./useGuildFetch";
import { useToast } from "./useToast";
import { welcomeService } from "@/services/welcomeService";
import type { WelcomeConfig, SaveWelcomeConfigParams } from "@/types/welcome";

// State module-scoped : un seul cache partage entre la page et le form
// organism. useGuildFetch est concu pour etre hisse au scope module.
const { guildIdFilter } = useGuildSelector();

const saving = ref(false);

const { data: config, loading, refresh: fetchConfig } = useGuildFetch<WelcomeConfig | null>(
  (guildId) => (guildId ? welcomeService.getConfig(guildId) : Promise.resolve(null)),
  null,
  { label: "configuration de bienvenue" },
);

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

  async function publishRules() {
    if (!guildIdFilter.value) return;
    try {
      await welcomeService.publishRules(guildIdFilter.value);
      success("Règlement publié dans le salon configuré.");
    } catch (e) {
      console.error("Erreur publication règlement :", e);
      showError("Impossible de publier le règlement (vérifie le salon et l'activation).");
      throw e;
    }
  }

  return { config, loading, saving, fetchConfig, saveConfig, publishRules };
}
