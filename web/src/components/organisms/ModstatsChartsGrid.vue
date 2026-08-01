<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { modstatsService } from "@/services/moderationAdvancedService";
import type { ModStatsEntry } from "@/types/moderation-advanced";
import type { ModstatsTrendDay } from "@/services/moderationAdvancedService";
import { Bar, Line } from "vue-chartjs";
import { registerChartJs } from "@/utils/chartjs";
import {
  makeLineOptions,
  makeBarOptions,
  colorAt,
  fillColor,
  severityColors,
} from "@/utils/chartTheme";

registerChartJs();

// Couleurs semantiques par type d'action (partagees entre tous les datasets).
const actionColors = {
  warns: severityColors.info,
  mutes: severityColors.medium,
  bans: severityColors.critical,
  kicks: colorAt(2),
} as const;

const props = defineProps<{
  days: number;
}>();

const emit = defineEmits<{
  refreshed: [];
}>();

const { guildIdFilter } = useGuildSelector();
const { error: showError } = useToast();
const stats = ref<ModStatsEntry[]>([]);
const trend = ref<ModstatsTrendDay[]>([]);
const loading = ref(true);

async function fetchStats() {
  if (!guildIdFilter.value) {
    stats.value = [];
    trend.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    const [s, t] = await Promise.all([
      modstatsService.list(guildIdFilter.value, props.days),
      modstatsService.trend(guildIdFilter.value, props.days).catch(() => [] as ModstatsTrendDay[]),
    ]);
    stats.value = s;
    trend.value = t;
  } catch (e) {
    console.error(e);
    showError("Erreur chargement modstats.");
  } finally {
    loading.value = false;
    emit("refreshed");
  }
}

defineExpose({ refresh: fetchStats });

onMounted(fetchStats);
watch([guildIdFilter, () => props.days], fetchStats);

// ── Charts ──
const top5Mods = computed(() => stats.value.slice(0, 5));

const lineOptions = makeLineOptions();

const trendData = computed(() => {
  const labels = trend.value.map((d) => {
    const dt = new Date(d.day);
    return `${dt.getDate()}/${dt.getMonth() + 1}`;
  });
  return {
    labels,
    datasets: [
      { label: "Avertissements", data: trend.value.map((d) => d.warns), borderColor: actionColors.warns, backgroundColor: fillColor(actionColors.warns), fill: true, tension: 0.3 },
      { label: "Sourdines", data: trend.value.map((d) => d.mutes), borderColor: actionColors.mutes, backgroundColor: fillColor(actionColors.mutes), fill: true, tension: 0.3 },
      { label: "Bannissements", data: trend.value.map((d) => d.bans), borderColor: actionColors.bans, backgroundColor: fillColor(actionColors.bans), fill: true, tension: 0.3 },
      { label: "Kicks", data: trend.value.map((d) => d.kicks), borderColor: actionColors.kicks, backgroundColor: fillColor(actionColors.kicks), fill: true, tension: 0.3 },
    ],
  };
});

const hasTrend = computed(() =>
  trend.value.some((d) => d.warns + d.mutes + d.bans + d.kicks > 0),
);

const horizontalBarOptions = makeBarOptions({}, true);

const stackedHorizontalOptions = makeBarOptions(
  {
    plugins: { legend: { display: true, position: "top" } },
    scales: { x: { stacked: true }, y: { stacked: true } },
  },
  true,
);

const topModsData = computed(() => ({
  labels: top5Mods.value.map((m) => m.moderator_name),
  datasets: [
    { label: "Total actions", data: top5Mods.value.map((m) => m.total), backgroundColor: colorAt(0), borderRadius: 6 },
  ],
}));

const totalsByType = computed(() => ({
  warns: stats.value.reduce((s, e) => s + e.warns, 0),
  mutes: stats.value.reduce((s, e) => s + e.mutes, 0),
  bans: stats.value.reduce((s, e) => s + e.bans, 0),
  kicks: stats.value.reduce((s, e) => s + e.kicks, 0),
}));

const distributionData = computed(() => ({
  labels: ["Avertissements", "Sourdines", "Bannissements", "Kicks"],
  datasets: [
    {
      label: "Total",
      data: [
        totalsByType.value.warns,
        totalsByType.value.mutes,
        totalsByType.value.bans,
        totalsByType.value.kicks,
      ],
      backgroundColor: [actionColors.warns, actionColors.mutes, actionColors.bans, actionColors.kicks],
      borderRadius: 6,
    },
  ],
}));

const breakdownData = computed(() => ({
  labels: top5Mods.value.map((m) => m.moderator_name),
  datasets: [
    { label: "Avertissements", data: top5Mods.value.map((m) => m.warns), backgroundColor: actionColors.warns },
    { label: "Sourdines", data: top5Mods.value.map((m) => m.mutes), backgroundColor: actionColors.mutes },
    { label: "Bannissements", data: top5Mods.value.map((m) => m.bans), backgroundColor: actionColors.bans },
    { label: "Kicks", data: top5Mods.value.map((m) => m.kicks), backgroundColor: actionColors.kicks },
  ],
}));

const topWarnsUsers = computed(() =>
  [...stats.value].filter((m) => m.warns > 0).sort((a, b) => b.warns - a.warns).slice(0, 5),
);
const topWarnsData = computed(() => ({
  labels: topWarnsUsers.value.map((m) => m.moderator_name),
  datasets: [{ label: "Avertissements", data: topWarnsUsers.value.map((m) => m.warns), backgroundColor: actionColors.warns, borderRadius: 6 }],
}));

const topMutesUsers = computed(() =>
  [...stats.value].filter((m) => m.mutes > 0).sort((a, b) => b.mutes - a.mutes).slice(0, 5),
);
const topMutesData = computed(() => ({
  labels: topMutesUsers.value.map((m) => m.moderator_name),
  datasets: [{ label: "Sourdines", data: topMutesUsers.value.map((m) => m.mutes), backgroundColor: actionColors.mutes, borderRadius: 6 }],
}));

const topHardUsers = computed(() =>
  [...stats.value]
    .map((m) => ({ ...m, hard: m.bans + m.kicks }))
    .filter((m) => m.hard > 0)
    .sort((a, b) => b.hard - a.hard)
    .slice(0, 5),
);
const topHardData = computed(() => ({
  labels: topHardUsers.value.map((m) => m.moderator_name),
  datasets: [{ label: "Bans + Kicks", data: topHardUsers.value.map((m) => m.hard), backgroundColor: actionColors.bans, borderRadius: 6 }],
}));

const percentBarOptions = makeBarOptions(
  {
    scales: { x: { ticks: { callback: (v: number | string) => `${v}%` } } },
  },
  true,
);

const severityData = computed(() => ({
  labels: top5Mods.value.map((m) => m.moderator_name),
  datasets: [{
    label: "Sévérité (%)",
    data: top5Mods.value.map((m) => {
      const t = m.total || 1;
      return Math.round(((m.bans + m.kicks) / t) * 1000) / 10;
    }),
    backgroundColor: actionColors.kicks,
    borderRadius: 6,
  }],
}));

const heatmapMod = computed(() => {
  if (top5Mods.value.length === 0) return null;
  const cols = ["warns", "mutes", "bans", "kicks"] as const;
  const labels = ["Avert.", "Sourd.", "Bans", "Kicks"];
  let max = 1;
  for (const m of top5Mods.value) {
    for (const c of cols) if (m[c] > max) max = m[c];
  }
  return { rows: top5Mods.value, cols, labels, max };
});

function heatColorMod(v: number, max: number): string {
  if (v === 0) return "rgba(88, 101, 242, 0.05)";
  const intensity = Math.min(v / max, 1);
  return `rgba(88, 101, 242, ${0.1 + intensity * 0.8})`;
}
</script>

<template>
  <section class="dash-section">
    <h2 class="section-title">Activité des modérateurs</h2>

    <div class="charts-grid">
      <div class="card chart-card">
        <h3>Tendance modération</h3>
        <div v-if="loading" class="chart-empty">Chargement…</div>
        <div v-else-if="!hasTrend" class="chart-empty">Aucune action sur les 30 derniers jours.</div>
        <div v-else class="chart-container">
          <Line :data="trendData" :options="lineOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Top 5 modérateurs (total)</h3>
        <div v-if="loading" class="chart-empty">Chargement…</div>
        <div v-else-if="stats.length === 0" class="chart-empty">Aucune action sur les 30 derniers jours.</div>
        <div v-else class="chart-container">
          <Bar :data="topModsData" :options="horizontalBarOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Répartition des actions</h3>
        <div v-if="loading" class="chart-empty">Chargement…</div>
        <div v-else-if="stats.length === 0" class="chart-empty">Aucune donnée.</div>
        <div v-else class="chart-container">
          <Bar :data="distributionData" :options="horizontalBarOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Détail par modérateur (top 5)</h3>
        <div v-if="loading" class="chart-empty">Chargement…</div>
        <div v-else-if="stats.length === 0" class="chart-empty">Aucune donnée.</div>
        <div v-else class="chart-container">
          <Bar :data="breakdownData" :options="stackedHorizontalOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Top 5 — Avertissements</h3>
        <div v-if="loading" class="chart-empty">Chargement…</div>
        <div v-else-if="topWarnsUsers.length === 0" class="chart-empty">Aucun avertissement.</div>
        <div v-else class="chart-container">
          <Bar :data="topWarnsData" :options="horizontalBarOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Top 5 — Sourdines</h3>
        <div v-if="loading" class="chart-empty">Chargement…</div>
        <div v-else-if="topMutesUsers.length === 0" class="chart-empty">Aucune sourdine.</div>
        <div v-else class="chart-container">
          <Bar :data="topMutesData" :options="horizontalBarOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Top 5 — Sanctions sévères (bans + kicks)</h3>
        <div v-if="loading" class="chart-empty">Chargement…</div>
        <div v-else-if="topHardUsers.length === 0" class="chart-empty">Aucune sanction sévère.</div>
        <div v-else class="chart-container">
          <Bar :data="topHardData" :options="horizontalBarOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Indice de sévérité (top 5)</h3>
        <div v-if="loading" class="chart-empty">Chargement…</div>
        <div v-else-if="stats.length === 0" class="chart-empty">Aucune donnée.</div>
        <div v-else class="chart-container">
          <Bar :data="severityData" :options="percentBarOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Heatmap mod × action</h3>
        <div v-if="loading" class="chart-empty">Chargement…</div>
        <div v-else-if="!heatmapMod" class="chart-empty">Aucune donnée.</div>
        <div v-else class="heatmap-wrapper">
          <table class="heatmap-table">
            <thead>
              <tr>
                <th></th>
                <th v-for="(l, i) in heatmapMod.labels" :key="i" class="heatmap-col">{{ l }}</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="m in heatmapMod.rows" :key="m.moderator_id">
                <td class="heatmap-mod">{{ m.moderator_name }}</td>
                <td
                  v-for="(c, i) in heatmapMod.cols"
                  :key="c"
                  class="heatmap-cell"
                  :style="{ backgroundColor: heatColorMod(m[c], heatmapMod.max) }"
                  :title="`${m.moderator_name} ${heatmapMod.labels[i]}: ${m[c]}`"
                ></td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.dash-section { margin-bottom: 32px; }
.section-title {
  position: relative;
  font-size: 14px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0 0 16px 0;
  padding: 0 0 8px 14px;
  border-bottom: 1px solid var(--border);
}
.section-title::before {
  content: "";
  position: absolute;
  left: 0;
  top: 2px;
  bottom: 14px;
  width: 3px;
  border-radius: var(--radius-xs);
  background: linear-gradient(to bottom,
    var(--accent),
    color-mix(in srgb, var(--accent) 50%, var(--accent-alt, #a855f7)));
}

.charts-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 20px;
  margin-bottom: 20px;
}
@media (max-width: 1300px) {
  .charts-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@media (max-width: 800px) {
  .charts-grid { grid-template-columns: 1fr; }
}

.chart-card {
  padding: var(--space-xl);
  min-width: 0;
  position: relative;
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  background: var(--bg-card);
  opacity: 0;
  animation: chart-card-enter 0.5s ease-out forwards;
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1),
    border-color 0.25s ease,
    box-shadow 0.3s ease;
}
.chart-card:hover {
  transform: translateY(-2px);
  border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
  box-shadow: 0 8px 22px color-mix(in srgb, var(--accent) 12%, transparent);
}

.chart-card:nth-child(1)  { animation-delay: 0.05s; }
.chart-card:nth-child(2)  { animation-delay: 0.10s; }
.chart-card:nth-child(3)  { animation-delay: 0.15s; }
.chart-card:nth-child(4)  { animation-delay: 0.20s; }
.chart-card:nth-child(5)  { animation-delay: 0.25s; }
.chart-card:nth-child(6)  { animation-delay: 0.30s; }
.chart-card:nth-child(7)  { animation-delay: 0.35s; }
.chart-card:nth-child(8)  { animation-delay: 0.40s; }
.chart-card:nth-child(9)  { animation-delay: 0.45s; }
.chart-card:nth-child(n+10) { animation-delay: 0.50s; }

@keyframes chart-card-enter {
  0%   { opacity: 0; transform: translateY(10px); }
  100% { opacity: 1; transform: translateY(0); }
}

.chart-card h3 {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
  margin-bottom: 16px;
}
.chart-container {
  height: 240px;
  position: relative;
}
.chart-empty {
  height: 240px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
  font-size: 13px;
}

.heatmap-wrapper { width: 100%; }
.heatmap-table {
  border-collapse: separate;
  border-spacing: 2px;
  width: 100%;
  table-layout: fixed;
}
.heatmap-col {
  font-size: 9px;
  color: var(--text-secondary);
  padding: 1px 0;
  text-align: center;
}
.heatmap-mod {
  font-size: 11px;
  color: var(--text-secondary);
  padding-right: 6px;
  white-space: nowrap;
  text-align: right;
  width: 60px;
  overflow: hidden;
  text-overflow: ellipsis;
}
.heatmap-cell {
  height: 24px;
  border-radius: var(--radius-xs);
  cursor: default;
}

@media (prefers-reduced-motion: reduce) {
  .chart-card {
    animation: none !important;
    opacity: 1;
    transform: none !important;
  }
  .chart-card:hover { transform: none; }
}
</style>
