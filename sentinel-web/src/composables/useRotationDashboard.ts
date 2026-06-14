import { ref, watch } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { rotationService, type RotationState, type ServedEntry } from "@/services/rotationService";

const { selectedGuildId } = useGuildSelector();
const state = ref<RotationState | null>(null);
const history = ref<ServedEntry[]>([]);
const loading = ref(true);

async function fetchData() {
  if (!selectedGuildId.value) {
    state.value = null;
    history.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    const [st, hist] = await Promise.all([
      rotationService.getState(selectedGuildId.value),
      rotationService.getHistory(selectedGuildId.value),
    ]);
    state.value = st;
    history.value = hist;
  } catch (e) {
    console.error("Erreur chargement rotation :", e);
    state.value = null;
    history.value = [];
  } finally {
    loading.value = false;
  }
}

watch(selectedGuildId, fetchData, { immediate: true });

export function useRotationDashboard() {
  return { state, history, loading, refresh: fetchData };
}
