import { ref, computed, watch } from "vue";
import { gamesService, type Game, type GamePanel } from "@/services/gamesService";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";

// State module-scoped : un seul cache partage entre la page et tous les
// organisms (table, panels, modal). Sans ca, chaque appel useGames()
// creerait son propre state et le filter category serait desync de la
// table affichee.
const { selectedGuildId } = useGuildSelector();

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
  const { error: showError } = useToast();
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

// Auto-fetch au changement de guild (le module est evalue au 1er import).
watch(selectedGuildId, fetchAll, { immediate: true });

export function useGames() {
  return {
    games,
    panels,
    categories,
    loading,
    fetchAll,
  };
}
