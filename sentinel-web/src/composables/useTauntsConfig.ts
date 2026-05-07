import { ref, watch } from "vue";
import { coudeService, type TauntsConfig } from "@/services/coudeService";
import { guildsService, type DiscordTextChannel } from "@/services/guildsService";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

// Singleton module-scoped : un cache partage entre ConfigCard et OptOutsCard.
const { selectedGuildId } = useGuildSelector();

const config = ref<TauntsConfig | null>(null);
const channels = ref<DiscordTextChannel[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

async function fetchConfig() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  error.value = null;
  try {
    const [cfg, chans] = await Promise.all([
      coudeService.getTauntsConfig(selectedGuildId.value),
      guildsService.getTextChannels(selectedGuildId.value),
    ]);
    config.value = cfg;
    channels.value = chans;
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

watch(selectedGuildId, fetchConfig, { immediate: true });

export function useTauntsConfig() {
  const { success, error: toastError } = useToast();

  async function save(payload: {
    channel_id: string | null;
    enabled: boolean;
    rename_enabled: boolean;
    messages_enabled: boolean;
  }) {
    if (!selectedGuildId.value) return;
    try {
      await coudeService.updateTauntsConfig(selectedGuildId.value, payload);
      success("Config railleries sauvegardee.");
      await fetchConfig();
    } catch (e) {
      toastError(String(e));
    }
  }

  async function removeOptOut(userId: string) {
    if (!selectedGuildId.value) return;
    try {
      await coudeService.removeTauntOptOut(selectedGuildId.value, userId);
      success("Opt-out retire.");
      await fetchConfig();
    } catch (e) {
      toastError(String(e));
    }
  }

  return { config, channels, loading, error, fetchConfig, save, removeOptOut };
}
