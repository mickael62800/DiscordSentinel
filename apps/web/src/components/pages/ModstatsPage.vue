<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { modstatsService } from "@/services/moderationAdvancedService";
import type { ModStatsEntry } from "@/types/moderation-advanced";
import { Bar } from "vue-chartjs";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  BarElement,
  Title,
  Tooltip,
  Legend,
} from "chart.js";

ChartJS.register(CategoryScale, LinearScale, BarElement, Title, Tooltip, Legend);

const { guildIdFilter } = useGuildSelector();
const { error: showError } = useToast();
const stats = ref<ModStatsEntry[]>([]);
const loading = ref(true);

async function fetchStats() {
  if (!guildIdFilter.value) {
    stats.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    stats.value = await modstatsService.list(guildIdFilter.value);
  } catch (e) {
    console.error(e);
    showError("Erreur chargement modstats.");
  } finally {
    loading.value = false;
  }
}

onMounted(fetchStats);
watch(guildIdFilter, fetchStats);

const totalActions = computed(() =>
  stats.value.reduce((acc, s) => acc + s.total, 0),
);
const activeMods = computed(() => stats.value.length);
const topMod = computed(() => stats.value[0]);

function medal(idx: number): string {
  return ["🥇", "🥈", "🥉"][idx] ?? `#${idx + 1}`;
}

// ── Charts ──
const top5Mods = computed(() => stats.value.slice(0, 5));

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
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>📊 Modstats — 30 derniers jours</h1>
      <p class="lede">
        Métriques d'activité par modérateur sur les 30 derniers jours :
        warns / mutes / bans / kicks. Top 20 trié par nombre total d'actions.
      </p>
    </header>

    <section class="kpi-row">
      <div class="kpi-card">
        <span class="kpi-value">{{ totalActions }}</span>
        <span class="kpi-label">Total actions</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ activeMods }}</span>
        <span class="kpi-label">Modérateurs actifs</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ topMod?.moderator_name ?? "—" }}</span>
        <span class="kpi-label">Top modérateur</span>
      </div>
    </section>

    <section class="charts-grid">
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
    </section>

    <section class="card">
      <h2>Classement</h2>
      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="stats.length === 0" class="empty">
        Aucune action de modération sur les 30 derniers jours.
      </div>
      <table v-else class="table">
        <thead>
          <tr>
            <th>#</th>
            <th>Modérateur</th>
            <th>Total</th>
            <th>Warns</th>
            <th>Mutes</th>
            <th>Bans</th>
            <th>Kicks</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(s, idx) in stats" :key="s.moderator_id">
            <td class="rank">{{ medal(idx) }}</td>
            <td>
              <strong>{{ s.moderator_name }}</strong>
              <small class="muted">{{ s.moderator_id }}</small>
            </td>
            <td><strong>{{ s.total }}</strong></td>
            <td>{{ s.warns }}</td>
            <td>{{ s.mutes }}</td>
            <td>{{ s.bans }}</td>
            <td>{{ s.kicks }}</td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.kpi-row {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin-bottom: 20px;
}
.kpi-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
}
.kpi-value {
  font-size: 1.6rem;
  font-weight: 700;
}
.kpi-label {
  font-size: 0.85rem;
  color: var(--text-secondary);
  margin-top: 4px;
}
.rank {
  font-size: 1.3rem;
  text-align: center;
  width: 60px;
}

/* Charts */
.charts-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 24px;
  margin-bottom: 24px;
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
</style>
