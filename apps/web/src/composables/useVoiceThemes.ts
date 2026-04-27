import { onMounted, ref, watch } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { voiceThemesService } from "@/services/voiceChannelsService";
import type {
  CreateThemePayload,
  UpdateThemePayload,
  VoiceChannelTheme,
} from "@/types/voice-extended";

export function useVoiceThemes() {
  const { guildIdFilter } = useGuildSelector();
  const { success, error: showError } = useToast();
  const themes = ref<VoiceChannelTheme[]>([]);
  const loading = ref(true);

  async function fetchThemes() {
    if (!guildIdFilter.value) {
      themes.value = [];
      loading.value = false;
      return;
    }
    loading.value = true;
    try {
      themes.value = await voiceThemesService.list(guildIdFilter.value);
    } catch (e) {
      console.error(e);
      showError("Erreur chargement thèmes.");
    } finally {
      loading.value = false;
    }
  }

  async function create(payload: CreateThemePayload) {
    if (!guildIdFilter.value) return;
    try {
      const created = await voiceThemesService.create(guildIdFilter.value, payload);
      themes.value.push(created);
      success("Thème créé.");
    } catch (e) {
      console.error(e);
      showError("Erreur création thème.");
      throw e;
    }
  }

  async function update(themeId: string, payload: UpdateThemePayload) {
    if (!guildIdFilter.value) return;
    try {
      const updated = await voiceThemesService.update(
        guildIdFilter.value,
        themeId,
        payload,
      );
      const idx = themes.value.findIndex((t) => t.id === themeId);
      if (idx !== -1) themes.value[idx] = updated;
      success("Thème mis à jour.");
    } catch (e) {
      console.error(e);
      showError("Erreur mise à jour thème.");
      throw e;
    }
  }

  async function remove(themeId: string) {
    if (!guildIdFilter.value) return;
    try {
      await voiceThemesService.remove(guildIdFilter.value, themeId);
      themes.value = themes.value.filter((t) => t.id !== themeId);
      success("Thème supprimé.");
    } catch (e) {
      console.error(e);
      showError("Erreur suppression thème.");
    }
  }

  onMounted(fetchThemes);
  watch(guildIdFilter, fetchThemes);

  return { themes, loading, fetchThemes, create, update, remove };
}
