<script setup lang="ts">
import { computed } from "vue";
import { useDashboard } from "../../composables/useDashboard";
import { useDashboardCharts } from "../../composables/useDashboardCharts";
import StatCard from "../molecules/StatCard.vue";
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

const { stats, loading: statsLoading } = useDashboard();
const { activity, loading: chartsLoading, days } = useDashboardCharts();

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

const labels = computed(() =>
  activity.value.map((a) => {
    const d = new Date(a.day);
    return `${d.getDate()}/${d.getMonth() + 1}`;
  }),
);

const messagesChartData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Messages",
      data: activity.value.map((a) => a.messages),
      borderColor: "#5865f2",
      backgroundColor: "rgba(88, 101, 242, 0.15)",
      fill: true,
      tension: 0.3,
    },
  ],
}));

const voiceChartData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Minutes vocales",
      data: activity.value.map((a) => a.voice_minutes),
      borderColor: "#57f287",
      backgroundColor: "rgba(87, 242, 135, 0.15)",
      fill: true,
      tension: 0.3,
    },
  ],
}));

const infractionsChartData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Warns",
      data: activity.value.map((a) => a.warns),
      backgroundColor: "#5bc0eb",
    },
    {
      label: "Mutes",
      data: activity.value.map((a) => a.mutes),
      backgroundColor: "#fee75c",
    },
    {
      label: "Bans",
      data: activity.value.map((a) => a.bans),
      backgroundColor: "#ed4245",
    },
  ],
}));

const totalInfractions = computed(() => {
  const w = activity.value.reduce((s, a) => s + a.warns, 0);
  const m = activity.value.reduce((s, a) => s + a.mutes, 0);
  const b = activity.value.reduce((s, a) => s + a.bans, 0);
  return { w, m, b };
});

const doughnutData = computed(() => ({
  labels: ["Warns", "Mutes", "Bans"],
  datasets: [
    {
      data: [totalInfractions.value.w, totalInfractions.value.m, totalInfractions.value.b],
      backgroundColor: ["#5bc0eb", "#fee75c", "#ed4245"],
      borderWidth: 0,
    },
  ],
}));

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

const membersChartData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Membres actifs",
      data: activity.value.map((a) => a.active_members),
      borderColor: "#fee75c",
      backgroundColor: "rgba(254, 231, 92, 0.15)",
      fill: true,
      tension: 0.3,
    },
  ],
}));
</script>

<template>
  <div class="dashboard">
    <div class="dashboard-header">
      <h1>Tableau de bord</h1>
      <div class="period-selector">
        <button :class="['period-btn', { active: days === 7 }]" @click="days = 7">7j</button>
        <button :class="['period-btn', { active: days === 14 }]" @click="days = 14">14j</button>
        <button :class="['period-btn', { active: days === 30 }]" @click="days = 30">30j</button>
        <button :class="['period-btn', { active: days === 90 }]" @click="days = 90">90j</button>
      </div>
    </div>

    <!-- Stats cards -->
    <div v-if="!statsLoading && stats" class="stats-grid">
      <StatCard label="Serveurs" :value="stats.total_servers" color="var(--accent)" />
      <StatCard label="Utilisateurs" :value="stats.total_users.toLocaleString()" color="var(--info)" />
      <StatCard label="Messages aujourd'hui" :value="stats.messages_today.toLocaleString()" />
      <StatCard label="Infractions aujourd'hui" :value="stats.infractions_today" color="var(--danger)" />
      <StatCard label="Bots en ligne" :value="`${stats.bots_online} / ${stats.bots_total}`" color="var(--success)" />
    </div>
    <div v-else-if="statsLoading" class="loading">Chargement des stats...</div>

    <!-- Charts -->
    <div v-if="!chartsLoading && activity.length > 0" class="charts-grid">
      <div class="chart-card">
        <h3>Messages</h3>
        <div class="chart-container">
          <Line :data="messagesChartData" :options="chartOptions" />
        </div>
      </div>

      <div class="chart-card">
        <h3>Activite vocale</h3>
        <div class="chart-container">
          <Line :data="voiceChartData" :options="chartOptions" />
        </div>
      </div>

      <div class="chart-card">
        <h3>Infractions</h3>
        <div class="chart-container">
          <Bar :data="infractionsChartData" :options="chartOptions" />
        </div>
      </div>

      <div class="chart-card">
        <h3>Repartition des infractions</h3>
        <div class="chart-container chart-container--small">
          <Doughnut :data="doughnutData" :options="doughnutOptions" />
        </div>
      </div>

      <div class="chart-card chart-card--wide">
        <h3>Membres actifs</h3>
        <div class="chart-container">
          <Line :data="membersChartData" :options="chartOptions" />
        </div>
      </div>
    </div>

    <div v-else-if="chartsLoading" class="loading">Chargement des graphiques...</div>
    <div v-else-if="activity.length === 0 && !chartsLoading" class="empty">
      Pas encore de donnees d'activite. Les graphiques apparaitront apres quelques heures d'utilisation.
    </div>
  </div>
</template>

<style scoped>
.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.dashboard-header h1 {
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

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
  margin-bottom: 24px;
}

.charts-grid {
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
  height: 220px;
  position: relative;
}

.chart-container--small {
  height: 200px;
  max-width: 300px;
  margin: 0 auto;
}

.loading, .empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
