import { ref, onMounted } from "vue";
import { systemService, type SystemInfo } from "@/services/systemService";

export function useSystemInfo() {
  const info = ref<SystemInfo | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);

  async function fetchInfo() {
    loading.value = true;
    error.value = null;
    try {
      info.value = await systemService.getInfo();
    } catch (e) {
      error.value = String(e);
      console.error("Erreur chargement system info :", e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchInfo);

  return { info, loading, error, fetchInfo };
}
