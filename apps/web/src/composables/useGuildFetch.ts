import { ref, onMounted, watch, type Ref, type WatchSource } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

/**
 * Variante guild-aware de useFetch. Le fetcher recoit l'id de guild courant
 * (ou null) et est appele a chaque changement de selection.
 */
export function useGuildFetch<T>(
  fetcher: (guildId: string | null) => Promise<T>,
  initialValue: T,
  options?: {
    guildScoped?: boolean;
    immediate?: boolean;
    watchSources?: WatchSource[];
    label?: string;
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
  const label = options?.label ?? "donnees";

  const guildScoped = options?.guildScoped ?? true;
  const immediate = options?.immediate ?? true;

  async function refresh() {
    loading.value = true;
    error.value = null;
    try {
      const guildId = guildScoped ? (guildIdFilter.value ?? null) : null;
      data.value = await fetcher(guildId);
    } catch (e) {
      const msg = String(e);
      if (msg.includes("Connection refused") || msg.includes("network") || msg.includes("connect")) {
        error.value = "Connexion au serveur impossible. Verifiez que l'API est demarree.";
      } else if (msg.includes("timeout") || msg.includes("Timeout")) {
        error.value = "Le serveur met trop de temps a repondre. Reessayez plus tard.";
      } else {
        error.value = "Erreur lors du chargement des donnees.";
      }
      console.error(`Echec du chargement ${label} :`, e);
      showError(error.value ?? `Echec du chargement ${label}.`);
    } finally {
      loading.value = false;
    }
  }

  if (immediate) onMounted(refresh);
  if (guildScoped) watch(guildIdFilter, refresh);
  if (options?.watchSources?.length) watch(options.watchSources, refresh);

  return { data, loading, error, refresh };
}
