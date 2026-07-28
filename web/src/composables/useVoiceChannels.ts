import { ref, computed, watch } from "vue";
import type { VoiceChannel, VoiceChannelDetail } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { voiceChannelsService, type VoiceChannelEvent } from "@/services/voiceChannelsService";

// State module-scoped : un seul cache partage entre la page et tous les
// organisms enfants (KPI bar / liste active / liste historique). Sans ca,
// chaque appel useVoiceChannels() creerait son propre state -> KPI desync.
const { selectedGuildId } = useGuildSelector();
const { data: channels, loading, error, refresh: fetchChannels } =
  useGuildFetch<VoiceChannel[]>(
    (guildId) => voiceChannelsService.getAll(guildId),
    [],
    { label: "canaux vocaux" },
  );

const historyChannels = ref<VoiceChannel[]>([]);
const historyLoading = ref(false);
const filterKind = ref("all");
const closing = ref<string | null>(null);
const cleaningAll = ref(false);
const purging = ref<string | null>(null);
const purgingAll = ref(false);

async function fetchHistory() {
  if (!selectedGuildId.value) {
    historyChannels.value = [];
    return;
  }
  historyLoading.value = true;
  try {
    historyChannels.value = await voiceChannelsService.getHistory(selectedGuildId.value, 100);
  } catch (e) {
    console.error("Erreur chargement historique salons vocaux:", e);
    historyChannels.value = [];
  } finally {
    historyLoading.value = false;
  }
}

// Charge l'historique au demarrage et au changement de guild.
watch(selectedGuildId, () => { fetchHistory(); }, { immediate: true });

const filteredChannels = computed(() => channels.value.filter((c) => {
  if (filterKind.value !== "all" && c.kind !== filterKind.value) return false;
  return true;
}));

const publicCount = computed(() => channels.value.filter((c) => c.kind === "public").length);
const privateCount = computed(() => channels.value.filter((c) => c.kind === "private").length);
const totalCount = computed(() => channels.value.length);

export function useVoiceChannels() {
  const { success, error: showError } = useToast();

  async function closeChannel(channelId: string): Promise<boolean> {
    closing.value = channelId;
    try {
      await voiceChannelsService.close(channelId);
      await Promise.all([fetchChannels(), fetchHistory()]);
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

  async function purgeAllHistory(): Promise<number> {
    if (!selectedGuildId.value) return 0;
    purgingAll.value = true;
    try {
      const res = await voiceChannelsService.purgeHistory(selectedGuildId.value);
      await fetchHistory();
      success(`${res.deleted} salon(s) supprime(s) de l'historique.`);
      return res.deleted;
    } catch (e) {
      console.error("Erreur purge historique:", e);
      showError("Erreur lors de la suppression de l'historique.");
      return 0;
    } finally {
      purgingAll.value = false;
    }
  }

  async function purgeChannel(channelId: string): Promise<boolean> {
    purging.value = channelId;
    try {
      await voiceChannelsService.purge(channelId);
      await fetchHistory();
      success("Salon supprime de l'historique.");
      return true;
    } catch (e) {
      console.error("Erreur purge salon vocal:", e);
      showError("Erreur lors de la suppression du salon.");
      return false;
    } finally {
      purging.value = null;
    }
  }

  async function closeAllDisplayed(): Promise<number> {
    cleaningAll.value = true;
    let success_count = 0;
    try {
      for (const ch of [...filteredChannels.value]) {
        try {
          await voiceChannelsService.close(ch.channel_id);
          success_count += 1;
        } catch (e) {
          console.warn(`Echec fermeture ${ch.channel_id}:`, e);
        }
      }
      await Promise.all([fetchChannels(), fetchHistory()]);
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
    historyChannels,
    historyLoading,
    fetchChannels,
    fetchHistory,
    closeChannel,
    closeAllDisplayed,
    purging,
    purgeChannel,
    purgingAll,
    purgeAllHistory,
  };
}

// State du detail panel (1 seul detail affiche a la fois).
const detail = ref<VoiceChannelDetail | null>(null);
const events = ref<VoiceChannelEvent[]>([]);
const detailLoading = ref(false);
const eventsLoading = ref(false);
const detailError = ref<string | null>(null);

export function useVoiceChannelDetail() {
  const { error: showError } = useToast();

  async function fetchDetail(channelId: string) {
    detailLoading.value = true;
    detailError.value = null;
    try {
      detail.value = await voiceChannelsService.getDetail(channelId);
    } catch (e) {
      detailError.value = String(e);
      showError("Erreur lors du chargement du detail du canal vocal.");
    } finally {
      detailLoading.value = false;
    }
    eventsLoading.value = true;
    try {
      events.value = await voiceChannelsService.getEvents(channelId);
    } catch (e) {
      console.warn("Erreur chargement timeline salon vocal:", e);
      events.value = [];
    } finally {
      eventsLoading.value = false;
    }
  }

  return {
    detail,
    events,
    loading: detailLoading,
    eventsLoading,
    error: detailError,
    fetchDetail,
  };
}
