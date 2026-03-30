import { ref, onMounted, watch, type Ref, type WatchSource } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useGuildSelector } from "./useGuildSelector";

export function useGuildFetch<T>(
  command: string,
  initialValue: T,
  options?: {
    extraParams?: Record<string, unknown> | (() => Record<string, unknown>);
    guildScoped?: boolean;
    immediate?: boolean;
    watchSources?: WatchSource[];
  },
): {
  data: Ref<T>;
  loading: Ref<boolean>;
  error: Ref<string | null>;
  refresh: () => Promise<void>;
} {
  const data = ref<T>(initialValue) as Ref<T>;
  const loading = ref(true);
  const error = ref<string | null>(null);
  const { guildIdFilter } = useGuildSelector();

  const guildScoped = options?.guildScoped ?? true;
  const immediate = options?.immediate ?? true;

  async function refresh() {
    loading.value = true;
    error.value = null;
    try {
      const extra = typeof options?.extraParams === "function"
        ? options.extraParams()
        : options?.extraParams ?? {};
      const params: Record<string, unknown> = {
        guildId: guildScoped ? (guildIdFilter.value ?? null) : null,
        ...extra,
      };
      data.value = await invoke<T>(command, params);
    } catch (e) {
      error.value = String(e);
      console.error(`Failed to invoke ${command}:`, e);
    } finally {
      loading.value = false;
    }
  }

  if (immediate) {
    onMounted(refresh);
  }

  if (guildScoped) {
    watch(guildIdFilter, refresh);
  }

  if (options?.watchSources?.length) {
    watch(options.watchSources, refresh);
  }

  return { data, loading, error, refresh };
}
