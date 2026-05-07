import { ref } from "vue";
import type { ModerationActionResponse, UserModerationHistory } from "../types";
import { useToast } from "./useToast";
import { moderationService, type LogActionParams } from "@/services/moderationService";

export function useModeration() {
  const { success, error: showError } = useToast();
  const submitting = ref(false);
  const history = ref<UserModerationHistory | null>(null);
  const historyLoading = ref(false);

  async function logAction(params: LogActionParams): Promise<ModerationActionResponse> {
    submitting.value = true;
    try {
      const result = await moderationService.logAction(params);
      success("Action de moderation enregistree avec succes.");
      return result;
    } catch (e) {
      console.error("Erreur lors de l'enregistrement de l'action de moderation :", e);
      showError("Erreur lors de l'enregistrement de l'action de moderation.");
      throw e;
    } finally {
      submitting.value = false;
    }
  }

  async function fetchHistory(guildId: string, userId: string) {
    historyLoading.value = true;
    try {
      history.value = await moderationService.getHistory(guildId, userId);
    } catch (e) {
      console.error("Erreur lors du chargement de l'historique de moderation :", e);
      showError("Erreur lors du chargement de l'historique de moderation.");
    } finally {
      historyLoading.value = false;
    }
  }

  return { submitting, history, historyLoading, logAction, fetchHistory };
}
