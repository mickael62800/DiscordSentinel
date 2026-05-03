import { computed, ref, watch } from "vue";
import { automodService } from "@/services/automodService";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import type { Infraction } from "@/types";
import { on as onWsEvent } from "@/api/events";

// State module-scoped : un seul cache partage entre la page et tous les
// organisms (StatsGrid, DetectionsTimeline). Sans ca, chaque appel
// useAutomod() creerait son propre state et le filter dans la timeline
// serait desync de la grid.
const { guildIdFilter } = useGuildSelector();

const detections = ref<Infraction[]>([]);
const loading = ref(true);
const userFilter = ref("");

async function fetchDetections() {
  const { error: showError } = useToast();
  if (!guildIdFilter.value) {
    detections.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    detections.value = await automodService.listDetections(guildIdFilter.value, {
      user_id: userFilter.value.trim() || undefined,
      limit: 100,
    });
  } catch (e) {
    console.error("Erreur chargement detections automod :", e);
    showError("Impossible de charger les detections.");
  } finally {
    loading.value = false;
  }
}

/** Stats agregees par categorie (champ `reason` parse). */
const statsByCategory = computed(() => {
  const counts = new Map<string, number>();
  for (const d of detections.value) {
    const cat = (d.reason || "autre").split(":")[0].trim().toLowerCase() || "autre";
    counts.set(cat, (counts.get(cat) ?? 0) + 1);
  }
  return Array.from(counts.entries())
    .sort((a, b) => b[1] - a[1])
    .map(([key, count]) => ({ key, count }));
});

/** Top users detectes (utile pour reperer les recidivistes). */
const topUsers = computed(() => {
  const counts = new Map<string, { username: string; count: number }>();
  for (const d of detections.value) {
    const cur = counts.get(d.user_id) ?? { username: d.username, count: 0 };
    cur.count += 1;
    counts.set(d.user_id, cur);
  }
  return Array.from(counts.entries())
    .map(([user_id, v]) => ({ user_id, ...v }))
    .sort((a, b) => b.count - a.count)
    .slice(0, 10);
});

const totalDetections = computed(() => detections.value.length);

// Auto-fetch au demarrage et au changement de guild + WS refresh live.
watch(guildIdFilter, fetchDetections, { immediate: true });
onWsEvent("ws:moderation_detection", () => fetchDetections());

export function useAutomod() {
  return {
    detections,
    statsByCategory,
    topUsers,
    totalDetections,
    loading,
    userFilter,
    fetchDetections,
  };
}
