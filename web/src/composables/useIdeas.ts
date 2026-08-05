import { computed, ref, watch } from "vue";

import {
  ideasService,
  type Idea,
  type IdeaDetail,
  type IdeaStatus,
} from "../services/ideasService";
import { useGuildSelector } from "./useGuildSelector";

/**
 * Liste des idees du serveur selectionne + filtres.
 *
 * Le filtre de statut est applique cote serveur (index dedie), la recherche
 * texte aussi : la page ne charge jamais tout pour filtrer ensuite.
 */
export function useIdeas() {
  const { guildIdFilter } = useGuildSelector();

  const ideas = ref<Idea[]>([]);
  const loading = ref(false);
  const error = ref<string | null>(null);

  const filterStatus = ref<IdeaStatus | "all">("all");
  const filterCategory = ref<string>("all");
  const filterSearch = ref("");

  const hasActiveFilters = computed(
    () =>
      filterStatus.value !== "all" ||
      filterCategory.value !== "all" ||
      filterSearch.value.trim() !== "",
  );

  const countByStatus = computed(() => {
    const counts: Record<string, number> = {};
    ideas.value.forEach((i) => {
      counts[i.status] = (counts[i.status] ?? 0) + 1;
    });
    return counts;
  });

  async function fetchIdeas() {
    if (!guildIdFilter.value) {
      ideas.value = [];
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      ideas.value = await ideasService.list({
        guild_id: guildIdFilter.value,
        status: filterStatus.value === "all" ? undefined : filterStatus.value,
        category: filterCategory.value === "all" ? undefined : filterCategory.value,
        search: filterSearch.value.trim() || undefined,
        limit: 200,
      });
    } catch (e) {
      console.error("Erreur chargement idees:", e);
      error.value = "Impossible de charger les idées";
    } finally {
      loading.value = false;
    }
  }

  function resetFilters() {
    filterStatus.value = "all";
    filterCategory.value = "all";
    filterSearch.value = "";
  }

  watch(
    [guildIdFilter, filterStatus, filterCategory],
    () => {
      void fetchIdeas();
    },
    { immediate: true },
  );

  return {
    ideas,
    loading,
    error,
    filterStatus,
    filterCategory,
    filterSearch,
    hasActiveFilters,
    countByStatus,
    fetchIdeas,
    resetFilters,
  };
}

/** Detail d'une idee (proposition + fil du salon). */
export function useIdeaDetail(ideaId: () => string | null) {
  const detail = ref<IdeaDetail | null>(null);
  const loading = ref(false);
  const error = ref<string | null>(null);
  const saving = ref(false);

  async function fetchDetail() {
    const id = ideaId();
    if (!id) {
      detail.value = null;
      return;
    }
    loading.value = true;
    error.value = null;
    try {
      detail.value = await ideasService.get(id);
    } catch (e) {
      console.error("Erreur chargement idee:", e);
      error.value = "Impossible de charger cette idée";
    } finally {
      loading.value = false;
    }
  }

  /**
   * Applique une decision. Renvoie l'erreur du domaine (transition interdite,
   * par exemple) plutot que de la masquer : l'appelant l'affiche en toast.
   */
  async function decide(status: IdeaStatus, reason: string) {
    const id = ideaId();
    if (!id) return;
    saving.value = true;
    try {
      const updated = await ideasService.decide(id, status, reason.trim() || undefined);
      if (detail.value) detail.value.idea = updated;
    } finally {
      saving.value = false;
    }
  }

  watch(ideaId, fetchDetail, { immediate: true });

  return { detail, loading, error, saving, fetchDetail, decide };
}
