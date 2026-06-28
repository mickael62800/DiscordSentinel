import { useGuildSelector } from "./useGuildSelector";
import { useGuildFetch } from "./useGuildFetch";
import { useToast } from "./useToast";
import { voiceThemesService } from "@/services/voiceChannelsService";
import type {
  CreateThemePayload,
  UpdateThemePayload,
  VoiceChannelTheme,
} from "@/types/voice-extended";

// Singleton module-scoped : un cache partage entre Table et FormModal.
// useGuildFetch est concu pour etre hisse au scope module (cache partage).
const { guildIdFilter } = useGuildSelector();

const { data: themes, loading, refresh: fetchThemes } = useGuildFetch<VoiceChannelTheme[]>(
  (guildId) => (guildId ? voiceThemesService.list(guildId) : Promise.resolve([])),
  [],
  { label: "themes vocaux" },
);

export function useVoiceThemes() {
  const { success, error: showError } = useToast();

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
      const updated = await voiceThemesService.update(guildIdFilter.value, themeId, payload);
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

  return { themes, loading, fetchThemes, create, update, remove };
}
