import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { ModerationActionResponse, UserModerationHistory } from "../types";

export function useModeration() {
  const submitting = ref(false);
  const history = ref<UserModerationHistory | null>(null);
  const historyLoading = ref(false);

  async function logAction(params: {
    guildId: string;
    channelId: string;
    moderatorId: string;
    moderatorName: string;
    targetId: string;
    targetName: string;
    actionType: string;
    reason: string;
    gravity?: string;
    duration?: number;
  }): Promise<ModerationActionResponse> {
    submitting.value = true;
    try {
      return await invoke<ModerationActionResponse>("log_moderation_action", params);
    } finally {
      submitting.value = false;
    }
  }

  async function fetchHistory(guildId: string, userId: string) {
    historyLoading.value = true;
    history.value = await invoke<UserModerationHistory>("get_moderation_history", {
      guildId,
      userId,
    });
    historyLoading.value = false;
  }

  return { submitting, history, historyLoading, logAction, fetchHistory };
}
