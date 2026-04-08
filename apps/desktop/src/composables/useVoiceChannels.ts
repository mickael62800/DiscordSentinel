import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { VoiceChannel, VoiceChannelDetail } from "../types";
import { useGuildFetch } from "./useGuildFetch";
import { useToast } from "./useToast";

export function useVoiceChannels() {
  const { data: channels, loading, error, refresh: fetchChannels } = useGuildFetch<VoiceChannel[]>(
    "get_voice_channels",
    [],
    { extraParams: {} },
  );

  const filterKind = ref("all");

  const filteredChannels = computed(() => {
    return channels.value.filter((c) => {
      if (filterKind.value !== "all" && c.kind !== filterKind.value) return false;
      return true;
    });
  });

  const publicCount = computed(() => channels.value.filter((c) => c.kind === "public").length);
  const privateCount = computed(() => channels.value.filter((c) => c.kind === "private").length);
  const totalCount = computed(() => channels.value.length);

  return { channels, filteredChannels, loading, error, filterKind, publicCount, privateCount, totalCount, fetchChannels };
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
      detail.value = await invoke<VoiceChannelDetail>("get_voice_channel_detail", { channelId });
    } catch (e) {
      error.value = String(e);
      showError("Erreur lors du chargement du detail du canal vocal.");
    } finally {
      loading.value = false;
    }
  }

  return { detail, loading, error, fetchDetail };
}
