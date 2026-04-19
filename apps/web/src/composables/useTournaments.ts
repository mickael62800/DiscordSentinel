import { ref } from "vue";
import {
  tournamentsService,
  type CurrentTournament,
  type PastTournament,
} from "@/services/tournamentsService";

export function useTournaments() {
  const current = ref<CurrentTournament | null>(null);
  const history = ref<PastTournament[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function loadCurrent(guildId: string) {
    loading.value = true;
    error.value = null;
    try {
      current.value = await tournamentsService.current(guildId);
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  }

  async function loadHistory(guildId: string) {
    loading.value = true;
    error.value = null;
    try {
      history.value = await tournamentsService.history(guildId);
    } catch (e: unknown) {
      error.value = e instanceof Error ? e.message : String(e);
    } finally {
      loading.value = false;
    }
  }

  async function loadAll(guildId: string) {
    await Promise.all([loadCurrent(guildId), loadHistory(guildId)]);
  }

  return { current, history, loading, error, loadCurrent, loadHistory, loadAll };
}
