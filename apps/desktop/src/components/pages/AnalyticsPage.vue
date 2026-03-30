<script setup lang="ts">
import { computed } from "vue";
import { useAnalytics } from "../../composables/useAnalytics";
import { Line, Bar, Doughnut } from "vue-chartjs";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  BarElement,
  ArcElement,
  Title,
  Tooltip,
  Legend,
  Filler,
} from "chart.js";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  BarElement,
  ArcElement,
  Title,
  Tooltip,
  Legend,
  Filler,
);

const { analytics, loading, days } = useAnalytics();

const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      labels: { color: "#9495b0", font: { size: 11 } },
    },
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

// ── Moderation Trend (Line) ──
const trendData = computed(() => {
  if (!analytics.value) return null;
  const t = analytics.value.moderation_trend;
  return {
    labels: t.map((d) => {
      const date = new Date(d.day);
      return `${date.getDate()}/${date.getMonth() + 1}`;
    }),
    datasets: [
      { label: "Avertissements", data: t.map((d) => d.warns), borderColor: "#5bc0eb", backgroundColor: "var(--info-bg)", fill: true, tension: 0.3 },
      { label: "Suppressions", data: t.map((d) => d.deletes), borderColor: "#ffa500", backgroundColor: "rgba(255,165,0,0.1)", fill: true, tension: 0.3 },
      { label: "Sourdines", data: t.map((d) => d.mutes), borderColor: "#fee75c", backgroundColor: "var(--warning-bg)", fill: true, tension: 0.3 },
      { label: "Bannissements", data: t.map((d) => d.bans), borderColor: "#ed4245", backgroundColor: "var(--danger-bg)", fill: true, tension: 0.3 },
    ],
  };
});

// ── Action Distribution (Doughnut) ──
const actionColors: Record<string, string> = {
  warn: "#5bc0eb",
  delete: "#ffa500",
  mute: "#fee75c",
  ban: "#ed4245",
};

const distributionData = computed(() => {
  if (!analytics.value) return null;
  const d = analytics.value.action_distribution;
  return {
    labels: d.map((a) => a.action),
    datasets: [{
      data: d.map((a) => a.count),
      backgroundColor: d.map((a) => actionColors[a.action] || "#888"),
      borderWidth: 0,
    }],
  };
});

const doughnutOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: {
      position: "bottom" as const,
      labels: { color: "#9495b0", font: { size: 11 }, padding: 16 },
    },
  },
};

// ── Top Infractors (Horizontal Bar) ──
const topInfractorsData = computed(() => {
  if (!analytics.value) return null;
  const t = analytics.value.top_infractors;
  return {
    labels: t.map((u) => u.username),
    datasets: [
      { label: "Avertissements", data: t.map((u) => u.warns), backgroundColor: "#5bc0eb" },
      { label: "Suppressions", data: t.map((u) => u.deletes), backgroundColor: "#ffa500" },
      { label: "Sourdines", data: t.map((u) => u.mutes), backgroundColor: "#fee75c" },
      { label: "Bannissements", data: t.map((u) => u.bans), backgroundColor: "#ed4245" },
    ],
  };
});

const stackedBarOptions = {
  ...chartOptions,
  indexAxis: "y" as const,
  plugins: {
    ...chartOptions.plugins,
    legend: { ...chartOptions.plugins.legend, position: "top" as const },
  },
  scales: {
    x: { ...chartOptions.scales.x, stacked: true },
    y: { ...chartOptions.scales.y, stacked: true },
  },
};

// ── Peak Hours (Bar) ──
const peakHoursData = computed(() => {
  if (!analytics.value) return null;
  const p = [...analytics.value.peak_hours].sort((a, b) => a.hour - b.hour);
  return {
    labels: p.map((h) => h.label),
    datasets: [
      {
        label: "Messages (moy.)",
        data: p.map((h) => h.avg_messages),
        backgroundColor: "rgba(88, 101, 242, 0.7)",
      },
      {
        label: "Infractions (moy.)",
        data: p.map((h) => h.avg_infractions),
        backgroundColor: "rgba(237, 66, 69, 0.7)",
      },
    ],
  };
});

// ── Heatmap (rendered as table) ──
const heatmapGrid = computed(() => {
  if (!analytics.value) return null;
  const points = analytics.value.heatmap;
  if (points.length === 0) return null;

  const dayNames = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche"];
  const hours = Array.from({ length: 24 }, (_, i) => i);

  // Build lookup
  const lookup = new Map<string, number>();
  let maxVal = 1;
  for (const p of points) {
    const key = `${p.day_of_week}-${p.hour}`;
    lookup.set(key, p.messages);
    if (p.messages > maxVal) maxVal = p.messages;
  }

  return { dayNames, hours, lookup, maxVal };
});

function heatColor(value: number, max: number): string {
  if (value === 0) return "rgba(88, 101, 242, 0.05)";
  const intensity = Math.min(value / max, 1);
  return `rgba(88, 101, 242, ${0.1 + intensity * 0.8})`;
}
</script>

<template>
  <div class="analytics-page">
    <div class="analytics-header">
      <h1>Analytics</h1>
      <div class="period-selector">
        <button :class="['period-btn', { active: days === 7 }]" @click="days = 7">7j</button>
        <button :class="['period-btn', { active: days === 14 }]" @click="days = 14">14j</button>
        <button :class="['period-btn', { active: days === 30 }]" @click="days = 30">30j</button>
        <button :class="['period-btn', { active: days === 90 }]" @click="days = 90">90j</button>
      </div>
    </div>

    <div v-if="loading" class="loading">Chargement des analytics...</div>

    <div v-else-if="analytics" class="analytics-grid">
      <!-- Moderation Trend -->
      <div class="chart-card chart-card--wide">
        <h3>Tendance de moderation</h3>
        <div class="chart-container">
          <Line v-if="trendData" :data="trendData" :options="chartOptions" />
        </div>
      </div>

      <!-- Action Distribution -->
      <div class="chart-card">
        <h3>Repartition des actions</h3>
        <div class="chart-container chart-container--small">
          <Doughnut v-if="distributionData" :data="distributionData" :options="doughnutOptions" />
        </div>
        <div v-if="analytics.action_distribution.length > 0" class="distribution-details">
          <div v-for="a in analytics.action_distribution" :key="a.action" class="dist-row">
            <span class="dist-dot" :style="{ background: actionColors[a.action] || '#888' }"></span>
            <span class="dist-label">{{ a.action }}</span>
            <span class="dist-count">{{ a.count }}</span>
            <span class="dist-pct">{{ a.percentage }}%</span>
          </div>
        </div>
      </div>

      <!-- Top Infractors -->
      <div class="chart-card">
        <h3>Top infracteurs</h3>
        <div class="chart-container">
          <Bar v-if="topInfractorsData" :data="topInfractorsData" :options="stackedBarOptions" />
        </div>
      </div>

      <!-- Peak Hours -->
      <div class="chart-card chart-card--wide">
        <h3>Activite par heure</h3>
        <div class="chart-container">
          <Bar v-if="peakHoursData" :data="peakHoursData" :options="chartOptions" />
        </div>
      </div>

      <!-- Heatmap -->
      <div v-if="heatmapGrid" class="chart-card chart-card--wide">
        <h3>Heatmap activite (messages)</h3>
        <div class="heatmap-wrapper">
          <table class="heatmap-table">
            <thead>
              <tr>
                <th></th>
                <th v-for="h in heatmapGrid.hours" :key="h" class="heatmap-hour">{{ h }}h</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(dayName, dayIdx) in heatmapGrid.dayNames" :key="dayIdx">
                <td class="heatmap-day">{{ dayName }}</td>
                <td
                  v-for="h in heatmapGrid.hours"
                  :key="h"
                  class="heatmap-cell"
                  :style="{ backgroundColor: heatColor(heatmapGrid.lookup.get(`${dayIdx}-${h}`) || 0, heatmapGrid.maxVal) }"
                  :title="`${dayName} ${h}h: ${heatmapGrid.lookup.get(`${dayIdx}-${h}`) || 0} msgs`"
                ></td>
              </tr>
            </tbody>
          </table>
        </div>
      </div>
    </div>

    <div v-else class="empty">Pas de donnees analytics disponibles.</div>
  </div>
</template>

<style scoped>
.analytics-page {
  padding: 0;
}

.analytics-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.analytics-header h1 {
  margin: 0;
}

.period-selector {
  display: flex;
  gap: 4px;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 3px;
}

.period-btn {
  padding: 6px 14px;
  border-radius: 6px;
  background: none;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
}

.period-btn.active {
  background-color: var(--accent);
  color: white;
}

.period-btn:hover:not(.active) {
  background-color: var(--bg-hover);
}

.analytics-grid {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 16px;
}

.chart-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 20px;
}

.chart-card--wide {
  grid-column: 1 / -1;
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
  height: 250px;
  position: relative;
}

.chart-container--small {
  height: 200px;
  max-width: 280px;
  margin: 0 auto;
}

/* Distribution details */
.distribution-details {
  margin-top: 16px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.dist-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.85rem;
}

.dist-dot {
  width: 10px;
  height: 10px;
  border-radius: 50%;
  flex-shrink: 0;
}

.dist-label {
  flex: 1;
  text-transform: capitalize;
}

.dist-count {
  font-weight: 600;
}

.dist-pct {
  color: var(--text-secondary);
  font-size: 0.8rem;
  min-width: 45px;
  text-align: right;
}

/* Heatmap table */
.heatmap-wrapper {
  overflow-x: auto;
}

.heatmap-table {
  border-collapse: collapse;
  width: 100%;
}

.heatmap-hour {
  font-size: 10px;
  color: var(--text-secondary);
  padding: 2px 0;
  text-align: center;
  min-width: 28px;
}

.heatmap-day {
  font-size: 11px;
  color: var(--text-secondary);
  padding-right: 8px;
  white-space: nowrap;
  text-align: right;
}

.heatmap-cell {
  width: 28px;
  height: 22px;
  border-radius: 3px;
  border: 1px solid var(--bg-card);
  cursor: default;
}

.loading, .empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
