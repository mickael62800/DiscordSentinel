import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { VoiceChannel, VoiceChannelDetail } from "../types";
import { useGuildSelector } from "./useGuildSelector";

export function useVoiceChannels() {
  const channels = ref<VoiceChannel[]>([]);
  const loading = ref(true);
  const filterKind = ref("all");
  const { guildIdFilter } = useGuildSelector();

  const filteredChannels = computed(() => {
    return channels.value.filter((c) => {
      if (filterKind.value !== "all" && c.kind !== filterKind.value) return false;
      return true;
    });
  });

  const publicCount = computed(() => channels.value.filter((c) => c.kind === "public").length);
  const privateCount = computed(() => channels.value.filter((c) => c.kind === "private").length);
  const totalCount = computed(() => channels.value.length);

  async function fetchChannels() {
    loading.value = true;
    try {
      channels.value = await invoke<VoiceChannel[]>("get_voice_channels", { guildId: guildIdFilter.value ?? "" });
    } catch (e) {
      console.error("Erreur chargement salons vocaux:", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchChannels);
  watch(guildIdFilter, fetchChannels);

  return { channels, filteredChannels, loading, filterKind, publicCount, privateCount, totalCount, fetchChannels };
}

export function useVoiceChannelDetail() {
  const detail = ref<VoiceChannelDetail | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function fetchDetail(channelId: string) {
    loading.value = true;
    error.value = null;
    try {
      detail.value = await invoke<VoiceChannelDetail>("get_voice_channel_detail", { channelId });
    } catch (e) {
      error.value = String(e);
    } finally {
      loading.value = false;
    }
  }

  return { detail, loading, error, fetchDetail };
}
