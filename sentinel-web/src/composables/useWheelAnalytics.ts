import { computed, ref, watch } from "vue";
import { useGuildSelector } from "./useGuildSelector";
import { useToast } from "./useToast";
import { wheelService } from "@/services/casinoService";
import type { WheelSpinLog, WheelTopWinner } from "@/types/casino";

// Singleton module-scoped : un cache partage entre KpiRow / Distribution / Top / Spins.
const { guildIdFilter } = useGuildSelector();

const spins = ref<WheelSpinLog[]>([]);
const topWinners = ref<WheelTopWinner[]>([]);
const loading = ref(true);

async function fetchAll() {
  const { error: showError } = useToast();
  if (!guildIdFilter.value) {
    spins.value = [];
    topWinners.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  const gid = guildIdFilter.value;
  try {
    const [s, t] = await Promise.all([
      wheelService.recentSpins(gid, 50).catch(() => []),
      wheelService.topWinners(gid, 7, 10).catch(() => []),
    ]);
    spins.value = s;
    topWinners.value = t;
  } catch (e) {
    console.error(e);
    showError("Erreur chargement wheel.");
  } finally {
    loading.value = false;
  }
}

const distribution = computed(() => {
  const counts = new Map<string, { label: string; count: number; total_payout: number }>();
  for (const s of spins.value) {
    const cur = counts.get(s.case_key) ?? { label: s.case_label, count: 0, total_payout: 0 };
    cur.count += 1;
    cur.total_payout += s.payout;
    counts.set(s.case_key, cur);
  }
  return Array.from(counts.entries())
    .map(([case_key, v]) => ({ case_key, ...v }))
    .sort((a, b) => b.count - a.count);
});

const totalSpins = computed(() => spins.value.length);
const totalPayout = computed(() => spins.value.reduce((a, b) => a + b.payout, 0));
const avgPayout = computed(() =>
  totalSpins.value > 0 ? Math.round(totalPayout.value / totalSpins.value) : 0,
);

watch(guildIdFilter, fetchAll, { immediate: true });

export function useWheelAnalytics() {
  return {
    spins, topWinners, loading,
    distribution, totalSpins, totalPayout, avgPayout,
    fetchAll,
  };
}
