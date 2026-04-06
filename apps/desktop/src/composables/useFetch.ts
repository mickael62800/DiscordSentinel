import { ref, onMounted, onUnmounted, type Ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

export function useFetch<T>(
  command: string,
  initialValue: T,
  params?: Record<string, unknown>,
): { data: Ref<T>; loading: Ref<boolean>; error: Ref<string | null>; refresh: () => Promise<void> } {
  const data = ref<T>(initialValue) as Ref<T>;
  const loading = ref(true);
  const error = ref<string | null>(null);
  let mounted = true;

  async function refresh() {
    loading.value = true;
    error.value = null;
    try {
      const result = params
        ? await invoke<T>(command, params)
        : await invoke<T>(command);
      if (mounted) {
        data.value = result;
      }
    } catch (e) {
      if (mounted) {
        error.value = String(e);
      }
      console.error(`Failed to invoke ${command}:`, e);
    } finally {
      if (mounted) {
        loading.value = false;
      }
    }
  }

  onMounted(refresh);
  onUnmounted(() => { mounted = false; });

  return { data, loading, error, refresh };
}
