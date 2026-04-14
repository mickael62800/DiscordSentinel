import { ref, computed } from "vue";
import type { VoiceChannel, VoiceChannelDetail } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { useToast } from "./useToast";
import { voiceChannelsService } from "@/services/voiceChannelsService";

export function useVoiceChannels() {
  const { success, error: showError } = useToast();
  const { data: channels, loading, error, refresh: fetchChannels } = useGuildFetch<VoiceChannel[]>(
    (guildId) => voiceChannelsService.getAll(guildId),
    [],
    { label: "canaux vocaux" },
  );

  const filterKind = ref("all");
  const closing = ref<string | null>(null);
  const cleaningAll = ref(false);

  const filteredChannels = computed(() => channels.value.filter((c) => {
    if (filterKind.value !== "all" && c.kind !== filterKind.value) return false;
    return true;
  }));

  const publicCount = computed(() => channels.value.filter((c) => c.kind === "public").length);
  const privateCount = computed(() => channels.value.filter((c) => c.kind === "private").length);
  const totalCount = computed(() => channels.value.length);

  async function closeChannel(channelId: string): Promise<boolean> {
    closing.value = channelId;
    try {
      await voiceChannelsService.close(channelId);
      await fetchChannels();
      success("Salon ferme et retire de la liste.");
      return true;
    } catch (e) {
      console.error("Erreur fermeture salon vocal:", e);
      showError("Erreur lors de la fermeture du salon vocal.");
      return false;
    } finally {
      closing.value = null;
    }
  }

  async function closeAllDisplayed(): Promise<number> {
    cleaningAll.value = true;
    let success_count = 0;
    try {
      // Sequentiel pour eviter de sature le backend. On ferme dans l'ordre
      // affiche et on refetch a la fin.
      for (const ch of [...filteredChannels.value]) {
        try {
          await voiceChannelsService.close(ch.channel_id);
          success_count += 1;
        } catch (e) {
          console.warn(`Echec fermeture ${ch.channel_id}:`, e);
        }
      }
      await fetchChannels();
      success(`${success_count} salon(s) ferme(s).`);
    } finally {
      cleaningAll.value = false;
    }
    return success_count;
  }

  return {
    channels,
    filteredChannels,
    loading,
    error,
    filterKind,
    publicCount,
    privateCount,
    totalCount,
    closing,
    cleaningAll,
    fetchChannels,
    closeChannel,
    closeAllDisplayed,
  };
}

export function useVoiceChannelDetail() {
  const { error: showError } = useToast();
  const detail = ref<VoiceChannelDetail | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function fetchDetail(channelId: string) {
    loading.value = true;
    error.value = null;
    try {
      detail.value = await voiceChannelsService.getDetail(channelId);
    } catch (e) {
      error.value = String(e);
      showError("Erreur lors du chargement du detail du canal vocal.");
    } finally {
      loading.value = false;
    }
  }

  return { detail, loading, error, fetchDetail };
}
