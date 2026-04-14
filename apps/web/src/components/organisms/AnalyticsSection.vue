<script setup lang="ts">
import { computed, toRef } from "vue";
import { useAnalytics } from "@/composables/useAnalytics";
import ErrorState from "../atoms/ErrorState.vue";
import { Line, Bar, Doughnut } from "vue-chartjs";

const props = defineProps<{ days: number }>();
const daysRef = toRef(props, "days");

const { analytics, loading, error, fetchAnalytics } = useAnalytics(
  computed({
    get: () => daysRef.value,
    set: () => {},
  }),
);

defineExpose({ refresh: fetchAnalytics });

const chartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  plugins: { legend: { labels: { color: "#9495b0", font: { size: 11 } } } },
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
  if (!analytics.value) return null;
  const t = analytics.value.moderation_trend;
  return {
    labels: t.map((d) => {
      const date = new Date(d.day);
      return `${date.getDate()}/${date.getMonth() + 1}`;
    }),
    datasets: [
      { label: "Avertissements", data: t.map((d) => d.warns), borderColor: "#5bc0eb", backgroundColor: "rgba(91, 192, 235, 0.15)", fill: true, tension: 0.3 },
      { label: "Suppressions", data: t.map((d) => d.deletes), borderColor: "#ffa500", backgroundColor: "rgba(255, 165, 0, 0.15)", fill: true, tension: 0.3 },
      { label: "Sourdines", data: t.map((d) => d.mutes), borderColor: "#fee75c", backgroundColor: "rgba(254, 231, 92, 0.15)", fill: true, tension: 0.3 },
      { label: "Bannissements", data: t.map((d) => d.bans), borderColor: "#ed4245", backgroundColor: "rgba(237, 66, 69, 0.15)", fill: true, tension: 0.3 },
    ],
  };
});

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
    datasets: [
      {
        data: d.map((a) => a.count),
        backgroundColor: d.map((a) => actionColors[a.action] || "#888"),
        borderWidth: 0,
      },
    ],
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

const peakHoursData = computed(() => {
  if (!analytics.value) return null;
  const p = [...analytics.value.peak_hours].sort((a, b) => a.hour - b.hour);
  return {
    labels: p.map((h) => h.label),
    datasets: [
      { label: "Messages (moy.)", data: p.map((h) => h.avg_messages), backgroundColor: "rgba(88, 101, 242, 0.7)" },
      { label: "Infractions (moy.)", data: p.map((h) => h.avg_infractions), backgroundColor: "rgba(237, 66, 69, 0.7)" },
    ],
  };
});

// ── Infractions par jour de la semaine (Bar empile) ──
// Agrege les warns/mutes/bans de moderation_trend par jour de la semaine.
// Complement naturel de la heatmap d'activite : l'une montre quand les gens
// parlent, l'autre quel jour genere le plus de moderation.
const weekdayInfractionsData = computed(() => {
  if (!analytics.value) return null;
  const dayNames = ["Dim", "Lun", "Mar", "Mer", "Jeu", "Ven", "Sam"];
  const warnSums = [0, 0, 0, 0, 0, 0, 0];
  const muteSums = [0, 0, 0, 0, 0, 0, 0];
  const banSums = [0, 0, 0, 0, 0, 0, 0];
  for (const t of analytics.value.moderation_trend) {
    const d = new Date(t.day).getDay();
    warnSums[d] += t.warns;
    muteSums[d] += t.mutes;
    banSums[d] += t.bans;
  }
  const order = [1, 2, 3, 4, 5, 6, 0];
  return {
    labels: order.map((i) => dayNames[i]),
    datasets: [
      { label: "Avertissements", data: order.map((i) => warnSums[i]), backgroundColor: "#5bc0eb" },
      { label: "Sourdines", data: order.map((i) => muteSums[i]), backgroundColor: "#fee75c" },
      { label: "Bannissements", data: order.map((i) => banSums[i]), backgroundColor: "#ed4245" },
    ],
  };
});

const weekdayBarOptions = {
  ...chartOptions,
  scales: {
    x: { ...chartOptions.scales.x, stacked: true },
    y: { ...chartOptions.scales.y, stacked: true },
  },
};

const heatmapGrid = computed(() => {
  if (!analytics.value) return null;
  const points = analytics.value.heatmap;
  if (points.length === 0) return null;

  const dayNames = ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"];
  const hours = Array.from({ length: 24 }, (_, i) => i);

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
  <section class="dash-section">
    <h2 class="section-title">Analytics de moderation</h2>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchAnalytics" />
    <div v-else-if="loading" class="loading">Chargement des analytics...</div>

    <div v-else-if="analytics" class="analytics-grid">
      <div class="chart-card">
        <h3>Tendance de moderation</h3>
        <div class="chart-container">
          <Line v-if="trendData" :data="trendData" :options="chartOptions" />
        </div>
      </div>

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

      <div class="chart-card">
        <h3>Top infracteurs</h3>
        <div class="chart-container">
          <Bar v-if="topInfractorsData" :data="topInfractorsData" :options="stackedBarOptions" />
        </div>
      </div>

      <div class="chart-card">
        <h3>Activite par heure</h3>
        <div class="chart-container">
          <Bar v-if="peakHoursData" :data="peakHoursData" :options="chartOptions" />
        </div>
      </div>

      <div v-if="heatmapGrid" class="chart-card">
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

      <div class="chart-card">
        <h3>Infractions par jour de la semaine</h3>
        <div class="chart-container">
          <Bar v-if="weekdayInfractionsData" :data="weekdayInfractionsData" :options="weekdayBarOptions" />
        </div>
      </div>
    </div>

    <div v-else class="empty">Pas de donnees analytics disponibles.</div>
  </section>
</template>

<style scoped>
.dash-section {
  margin-bottom: 32px;
}

.section-title {
  font-size: 14px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0 0 14px 2px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border);
}

.analytics-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
}

@media (max-width: 1300px) {
  .analytics-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 800px) {
  .analytics-grid {
    grid-template-columns: 1fr;
  }
}

.chart-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 20px;
  min-width: 0;
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
  height: 250px;
  position: relative;
}

.chart-container--small {
  height: 200px;
  max-width: 280px;
  margin: 0 auto;
}

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

.heatmap-wrapper { width: 100%; }

.heatmap-table {
  border-collapse: separate;
  border-spacing: 2px;
  width: 100%;
  /* fixed layout : la premiere ligne dicte les largeurs de colonne,
     le reste de la table s'aligne dessus. Sans ca, les cellules vides
     restent a leur taille minimum et la table ne remplit pas la carte. */
  table-layout: fixed;
}

.heatmap-hour {
  font-size: 9px;
  color: var(--text-secondary);
  padding: 1px 0;
  text-align: center;
}

.heatmap-day {
  font-size: 11px;
  color: var(--text-secondary);
  padding-right: 6px;
  white-space: nowrap;
  text-align: right;
  width: 36px;
}

.heatmap-cell {
  height: 24px;
  border-radius: 3px;
  cursor: default;
}

.loading,
.empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
