import { ref, onMounted, watch, type Ref, type WatchSource } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

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
  const { error: showError } = useToast();
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
      const msg = String(e);
      if (msg.includes("Connection refused") || msg.includes("network") || msg.includes("connect")) {
        error.value = "Connexion au serveur impossible. Verifiez que l'API est demarree.";
      } else if (msg.includes("timeout") || msg.includes("Timeout")) {
        error.value = "Le serveur met trop de temps a repondre. Reessayez plus tard.";
      } else {
        error.value = "Erreur lors du chargement des donnees.";
      }
      console.error(`Echec de l'appel ${command} :`, e);
      showError(error.value ?? `Echec de l'appel ${command}.`);
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
