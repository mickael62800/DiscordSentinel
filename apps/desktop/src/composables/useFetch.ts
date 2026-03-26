import { ref, onMounted, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export function useFetch<T>(
  command: string,
  initialValue: T,
  params?: Record<string, unknown>,
): { data: Ref<T>; loading: Ref<boolean>; error: Ref<string | null>; refresh: () => Promise<void> } {
  const data = ref<T>(initialValue) as Ref<T>;
  const loading = ref(true);
  const error = ref<string | null>(null);

  async function refresh() {
    loading.value = true;
    error.value = null;
    try {
      data.value = params
        ? await invoke<T>(command, params)
        : await invoke<T>(command);
    } catch (e) {
      error.value = String(e);
      console.error(`Failed to invoke ${command}:`, e);
    } finally {
      loading.value = false;
    }
  }

  onMounted(refresh);

  return { data, loading, error, refresh };
}
