<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { modstatsService } from "@/services/moderationAdvancedService";
import type { ModStatsEntry } from "@/types/moderation-advanced";
import type { ModstatsTrendDay } from "@/services/moderationAdvancedService";
import { Bar, Line } from "vue-chartjs";
import { registerChartJs } from "@/utils/chartjs";

registerChartJs();

const { guildIdFilter } = useGuildSelector();
const { error: showError } = useToast();
const stats = ref<ModStatsEntry[]>([]);
const trend = ref<ModstatsTrendDay[]>([]);
const loading = ref(true);
const refreshing = ref(false);
const days = ref(30);
const periods = computed(() => [7, 14, 30, 90]);

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
      modstatsService.list(guildIdFilter.value, days.value),
      modstatsService.trend(guildIdFilter.value, days.value).catch(() => [] as ModstatsTrendDay[]),
    ]);
    stats.value = s;
    trend.value = t;
  } catch (e) {
    console.error(e);
    showError("Erreur chargement modstats.");
  } finally {
    loading.value = false;
  }
}

async function handleRefresh() {
  refreshing.value = true;
  try {
    await fetchStats();
  } finally {
    refreshing.value = false;
  }
}

onMounted(fetchStats);
watch([guildIdFilter, days], fetchStats);

// ── Charts ──
const top5Mods = computed(() => stats.value.slice(0, 5));

const lineOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { labels: { color: "#9495b0", font: { size: 11 } } },
  },
  scales: {
    x: {
      ticks: { color: "#9495b0", font: { size: 10 } },
      grid: { color: "rgba(58, 59, 92, 0.5)" },
    },
    y: {
      ticks: { color: "#9495b0", font: { size: 10 } },
      grid: { color: "rgba(58, 59, 92, 0.5)" },
      beginAtZero: true,
    },
  },
};

const trendData = computed(() => {
  const labels = trend.value.map((d) => {
    const dt = new Date(d.day);
    return `${dt.getDate()}/${dt.getMonth() + 1}`;
  });
  return {
    labels,
    datasets: [
      {
        label: "Avertissements",
        data: trend.value.map((d) => d.warns),
        borderColor: "#5bc0eb",
        backgroundColor: "rgba(91, 192, 235, 0.15)",
        fill: true,
        tension: 0.3,
      },
      {
        label: "Sourdines",
        data: trend.value.map((d) => d.mutes),
        borderColor: "#fee75c",
        backgroundColor: "rgba(254, 231, 92, 0.15)",
        fill: true,
        tension: 0.3,
      },
      {
        label: "Bannissements",
        data: trend.value.map((d) => d.bans),
        borderColor: "#ed4245",
        backgroundColor: "rgba(237, 66, 69, 0.15)",
        fill: true,
        tension: 0.3,
      },
      {
        label: "Kicks",
        data: trend.value.map((d) => d.kicks),
        borderColor: "#e67e22",
        backgroundColor: "rgba(230, 126, 34, 0.15)",
        fill: true,
        tension: 0.3,
      },
    ],
  };
});

const hasTrend = computed(() =>
  trend.value.some((d) => d.warns + d.mutes + d.bans + d.kicks > 0),
);

const horizontalBarOptions = {
  responsive: true,
  maintainAspectRatio: false,
  indexAxis: "y" as const,
  plugins: { legend: { display: false } },
  scales: {
    x: {
      ticks: { color: "#9495b0", font: { size: 10 } },
      grid: { color: "rgba(58, 59, 92, 0.5)" },
      beginAtZero: true,
    },
    y: {
      ticks: { color: "#9495b0", font: { size: 11 } },
      grid: { display: false },
    },
  },
};

const stackedHorizontalOptions = {
  ...horizontalBarOptions,
  plugins: {
    legend: { labels: { color: "#9495b0", font: { size: 11 } }, position: "top" as const },
  },
  scales: {
    x: { ...horizontalBarOptions.scales.x, stacked: true },
    y: { ...horizontalBarOptions.scales.y, stacked: true },
  },
};

const topModsData = computed(() => ({
  labels: top5Mods.value.map((m) => m.moderator_name),
  datasets: [
    {
      label: "Total actions",
      data: top5Mods.value.map((m) => m.total),
      backgroundColor: "#5865f2",
      borderRadius: 6,
    },
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
      backgroundColor: ["#5bc0eb", "#fee75c", "#ed4245", "#e67e22"],
      borderRadius: 6,
    },
  ],
}));

const breakdownData = computed(() => ({
  labels: top5Mods.value.map((m) => m.moderator_name),
  datasets: [
    {
      label: "Avertissements",
      data: top5Mods.value.map((m) => m.warns),
      backgroundColor: "#5bc0eb",
    },
    {
      label: "Sourdines",
      data: top5Mods.value.map((m) => m.mutes),
      backgroundColor: "#fee75c",
    },
    {
      label: "Bannissements",
      data: top5Mods.value.map((m) => m.bans),
      backgroundColor: "#ed4245",
    },
    {
      label: "Kicks",
      data: top5Mods.value.map((m) => m.kicks),
      backgroundColor: "#e67e22",
    },
  ],
}));

// ── 4. Top 5 par avertissements ──
const topWarnsUsers = computed(() =>
  [...stats.value].filter((m) => m.warns > 0).sort((a, b) => b.warns - a.warns).slice(0, 5),
);
const topWarnsData = computed(() => ({
  labels: topWarnsUsers.value.map((m) => m.moderator_name),
  datasets: [{ label: "Avertissements", data: topWarnsUsers.value.map((m) => m.warns), backgroundColor: "#5bc0eb", borderRadius: 6 }],
}));

// ── 5. Top 5 par sourdines ──
const topMutesUsers = computed(() =>
  [...stats.value].filter((m) => m.mutes > 0).sort((a, b) => b.mutes - a.mutes).slice(0, 5),
);
const topMutesData = computed(() => ({
  labels: topMutesUsers.value.map((m) => m.moderator_name),
  datasets: [{ label: "Sourdines", data: topMutesUsers.value.map((m) => m.mutes), backgroundColor: "#fee75c", borderRadius: 6 }],
}));

// ── 6. Top 5 sanctions sevrres (bans + kicks) ──
const topHardUsers = computed(() =>
  [...stats.value]
    .map((m) => ({ ...m, hard: m.bans + m.kicks }))
    .filter((m) => m.hard > 0)
    .sort((a, b) => b.hard - a.hard)
    .slice(0, 5),
);
const topHardData = computed(() => ({
  labels: topHardUsers.value.map((m) => m.moderator_name),
  datasets: [{ label: "Bans + Kicks", data: topHardUsers.value.map((m) => m.hard), backgroundColor: "#ed4245", borderRadius: 6 }],
}));

const percentBarOptions = {
  ...horizontalBarOptions,
  scales: {
    ...horizontalBarOptions.scales,
    x: {
      ...horizontalBarOptions.scales.x,
      ticks: { ...horizontalBarOptions.scales.x.ticks, callback: (v: number | string) => `${v}%` },
    },
  },
};

// ── 7. Indice de sévérité (% actions sévères = bans+kicks) ──
const severityData = computed(() => ({
  labels: top5Mods.value.map((m) => m.moderator_name),
  datasets: [{
    label: "Sévérité (%)",
    data: top5Mods.value.map((m) => {
      const t = m.total || 1;
      return Math.round(((m.bans + m.kicks) / t) * 1000) / 10;
    }),
    backgroundColor: "#e67e22",
    borderRadius: 6,
  }],
}));

// ── 9. Heatmap mod × action ──
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
  <div class="dashboard">
    <div class="dashboard-header">
      <h1>Statistiques admin</h1>
      <div class="header-actions">
        <div class="period-selector">
          <button
            v-for="p in periods"
            :key="p"
            :class="['period-btn', { active: days === p }]"
            @click="days = p"
          >
            {{ p }}j
          </button>
        </div>
        <button
          class="refresh-btn"
          :disabled="refreshing"
          :title="refreshing ? 'Actualisation en cours…' : 'Actualiser les données'"
          @click="handleRefresh"
        >
          <svg
            :class="['refresh-icon', { spinning: refreshing }]"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
          >
            <path d="M3 12a9 9 0 0 1 15-6.7L21 8" />
            <path d="M21 3v5h-5" />
            <path d="M21 12a9 9 0 0 1-15 6.7L3 16" />
            <path d="M3 21v-5h5" />
          </svg>
          <span>Actualiser</span>
        </button>
      </div>
    </div>

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
  </div>
</template>

<style scoped>
/* Header (mirroir exact de StatsPage) */
.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
  margin-bottom: 24px;
  padding-bottom: 18px;
  border-bottom: 1px solid transparent;
  background:
    linear-gradient(to right,
      transparent 0%,
      color-mix(in srgb, var(--accent) 35%, transparent) 30%,
      color-mix(in srgb, var(--accent) 35%, transparent) 70%,
      transparent 100%) bottom / 100% 1px no-repeat;
}
.dashboard-header h1 {
  margin: 0;
  font-size: 1.6rem;
  font-weight: 700;
  background: linear-gradient(
    90deg,
    var(--text-primary) 0%,
    color-mix(in srgb, var(--accent) 60%, var(--text-primary)) 50%,
    var(--text-primary) 100%
  );
  background-size: 200% auto;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  color: transparent;
  animation: stats-title-shimmer 10s linear infinite;
  letter-spacing: 0.3px;
}
@keyframes stats-title-shimmer {
  0%   { background-position: 200% center; }
  100% { background-position: -200% center; }
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

/* ── Période selector — segmented control polished ──────── */
.period-selector {
  display: flex;
  gap: 2px;
  background-color: color-mix(in srgb, var(--bg-card) 80%, transparent);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 3px;
  position: relative;
  box-shadow:
    inset 0 1px 2px rgba(0, 0, 0, 0.18),
    0 1px 0 color-mix(in srgb, white 6%, transparent);
}
.period-btn {
  position: relative;
  padding: 6px 14px;
  border-radius: 7px;
  background: none;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease,
    background 0.25s ease,
    box-shadow 0.25s ease;
}
.period-btn::after {
  content: "";
  position: absolute;
  left: 50%;
  bottom: 3px;
  width: 0;
  height: 2px;
  border-radius: 2px;
  background: var(--accent);
  transform: translateX(-50%);
  transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}
.period-btn:hover:not(.active) {
  color: var(--text-primary);
  background-color: color-mix(in srgb, var(--accent) 8%, transparent);
}
.period-btn:hover:not(.active)::after {
  width: 60%;
}
.period-btn.active {
  background: linear-gradient(135deg,
    var(--accent),
    color-mix(in srgb, var(--accent) 75%, var(--accent-alt, #a855f7)));
  color: white;
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 35%, transparent),
    inset 0 -1px 0 color-mix(in srgb, black 15%, transparent),
    0 2px 8px color-mix(in srgb, var(--accent) 30%, transparent);
  text-shadow: 0 1px 1px rgba(0, 0, 0, 0.12);
}
.period-btn:active {
  transform: scale(0.96);
  transition-duration: 0.08s;
}

/* ── Refresh button polished ───────────────────────────── */
.refresh-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 7px 14px;
  border-radius: 10px;
  background:
    linear-gradient(180deg,
      color-mix(in srgb, white 4%, var(--bg-card)),
      var(--bg-card));
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease,
    background 0.25s ease,
    border-color 0.2s ease,
    box-shadow 0.25s ease;
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 6%, transparent);
}
.refresh-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--accent) 10%, var(--bg-card)),
    color-mix(in srgb, var(--accent) 6%, var(--bg-card)));
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 10%, transparent),
    0 4px 12px color-mix(in srgb, var(--accent) 18%, transparent);
}
.refresh-btn:hover:not(:disabled) .refresh-icon:not(.spinning) {
  transform: rotate(180deg);
}
.refresh-btn:active:not(:disabled) {
  transform: scale(0.97);
  transition-duration: 0.08s;
}
.refresh-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.refresh-icon {
  width: 14px;
  height: 14px;
  transition: transform 0.45s cubic-bezier(0.4, 0, 0.2, 1);
}
.refresh-icon.spinning { animation: spin 0.9s linear infinite; }
@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}

/* Section avec barre verticale accent à gauche */
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
  border-radius: 2px;
  background: linear-gradient(to bottom,
    var(--accent),
    color-mix(in srgb, var(--accent) 50%, var(--accent-alt, #a855f7)));
}

/* Charts */
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
  border-radius: 12px;
  background: var(--bg-card);
  /* Stagger entrance + hover cosy. */
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

/* Stagger : 9 cartes + heatmap = jusqu'a 10. */
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

.chart-card--wide { grid-column: 1 / -1; }
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

/* Heatmap mod × action — identique a StatsPage */
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
  border-radius: 3px;
  cursor: default;
}

@media (prefers-reduced-motion: reduce) {
  .dashboard-header h1 {
    animation: none;
    background: none;
    -webkit-text-fill-color: var(--text-primary);
    color: var(--text-primary);
  }
  .period-btn,
  .period-btn:hover,
  .period-btn:active,
  .refresh-btn,
  .refresh-btn:hover,
  .refresh-btn:active { transform: none; }
  .refresh-icon { transition: none !important; }
  .period-btn::after { transition: none !important; }
  .chart-card {
    animation: none !important;
    opacity: 1;
    transform: none !important;
  }
  .chart-card:hover { transform: none; }
}
</style>
