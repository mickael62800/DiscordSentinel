import { onMounted, ref, watch } from "vue";
import { strikesService } from "@/services/strikesService";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import type { StrikeConfig, UserStrike, SaveStrikeConfigPayload } from "@/types/strikes";

export function useStrikes() {
  const { guildIdFilter } = useGuildSelector();
  const { success, error: showError } = useToast();

  const config = ref<StrikeConfig | null>(null);
  const userStrikes = ref<UserStrike[]>([]);
  const lookupUserId = ref("");
  const loadingConfig = ref(true);
  const loadingStrikes = ref(false);
  const saving = ref(false);

  async function fetchConfig() {
    if (!guildIdFilter.value) {
      config.value = null;
      loadingConfig.value = false;
      return;
    }
    loadingConfig.value = true;
    try {
      config.value = await strikesService.getConfig(guildIdFilter.value);
    } catch (e) {
      console.error("Erreur chargement config strikes :", e);
      showError("Impossible de charger la config strikes.");
    } finally {
      loadingConfig.value = false;
    }
  }

  async function saveConfig(payload: SaveStrikeConfigPayload) {
    if (!guildIdFilter.value) return;
    saving.value = true;
    try {
      config.value = await strikesService.saveConfig(guildIdFilter.value, payload);
      success("Config strikes enregistrée.");
    } catch (e) {
      console.error("Erreur sauvegarde config strikes :", e);
      showError("Erreur lors de la sauvegarde.");
    } finally {
      saving.value = false;
    }
  }

  async function lookupStrikes() {
    if (!guildIdFilter.value || !lookupUserId.value.trim()) {
      userStrikes.value = [];
      return;
    }
    loadingStrikes.value = true;
    try {
      userStrikes.value = await strikesService.getActiveStrikes(
        guildIdFilter.value,
        lookupUserId.value.trim(),
      );
    } catch (e) {
      console.error("Erreur recherche strikes :", e);
      showError("Impossible de charger les strikes du user.");
      userStrikes.value = [];
    } finally {
      loadingStrikes.value = false;
    }
  }

  async function resetStrikes() {
    if (!guildIdFilter.value || !lookupUserId.value.trim()) return;
    try {
      await strikesService.resetStrikes(guildIdFilter.value, lookupUserId.value.trim());
      userStrikes.value = [];
      success("Strikes réinitialisés.");
    } catch (e) {
      console.error("Erreur reset strikes :", e);
      showError("Erreur lors du reset.");
    }
  }

  onMounted(fetchConfig);
  watch(guildIdFilter, fetchConfig);

  return {
    config,
    userStrikes,
    lookupUserId,
    loadingConfig,
    loadingStrikes,
    saving,
    fetchConfig,
    saveConfig,
    lookupStrikes,
    resetStrikes,
  };
}
