import { ref, computed, watch, onMounted } from "vue";
import { gamesService, type Game, type GamePanel } from "@/services/gamesService";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

export function useGames() {
  const { selectedGuildId } = useGuildSelector();
  const { error: showError } = useToast();

  const games = ref<Game[]>([]);
  const panels = ref<GamePanel[]>([]);
  const loading = ref(false);

  const categories = computed<string[]>(() => {
    const set = new Set<string>();
    for (const g of games.value) {
      if (g.category && g.category.trim()) set.add(g.category);
    }
    return Array.from(set).sort((a, b) => a.localeCompare(b));
  });

  async function fetchAll() {
    const gid = selectedGuildId.value;
    if (!gid) {
      games.value = [];
      panels.value = [];
      return;
    }
    loading.value = true;
    try {
      const [gs, ps] = await Promise.all([
        gamesService.list(gid),
        gamesService.listPanels(gid),
      ]);
      games.value = gs;
      panels.value = ps;
    } catch (e) {
      console.error("Erreur chargement jeux :", e);
      showError("Erreur lors du chargement des jeux.");
    } finally {
      loading.value = false;
    }
  }

  onMounted(fetchAll);
  watch(selectedGuildId, fetchAll);

  return {
    games,
    panels,
    categories,
    loading,
    fetchAll,
  };
}
