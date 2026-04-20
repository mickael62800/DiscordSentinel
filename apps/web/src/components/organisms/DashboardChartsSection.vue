<script setup lang="ts">
import { computed, toRef } from "vue";
import { useDashboardCharts } from "@/composables/useDashboardCharts";
import { Line, Bar, Doughnut } from "vue-chartjs";
import type { ScriptableContext } from "chart.js";

const props = defineProps<{ days: number }>();
const daysRef = toRef(props, "days");

const { activity, topUsers, loading, error, fetchAll } = useDashboardCharts(
  // On passe un ref derive du prop pour que le composable l'utilise comme source
  computed({
    get: () => daysRef.value,
    // pas de setter : la periode est pilotee par le parent
    set: () => {},
  }),
);

defineExpose({ refresh: fetchAll });

const chartOptions = {
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

const voiceChartData = computed(() => {
  let cumul = 0;
  const cumulData = activity.value.map((a) => {
    cumul += Math.round((a.voice_minutes / 60) * 10) / 10;
    return cumul;
  });
  return {
    labels: labels.value,
    datasets: [
      {
        label: "Heures vocales (cumule)",
        data: cumulData,
        borderColor: "#57f287",
        backgroundColor: "rgba(87, 242, 135, 0.12)",
        fill: true,
        tension: 0.3,
      },
      {
        label: "Heures / jour",
        data: activity.value.map((a) => Math.round((a.voice_minutes / 60) * 10) / 10),
        borderColor: "#2ecc71",
        backgroundColor: "rgba(46, 204, 113, 0.4)",
        type: "bar" as const,
      },
    ],
  } as any;
});

const infractionsChartData = computed(() => ({
  labels: labels.value,
  datasets: [
    { label: "Avertissements", data: activity.value.map((a) => a.warns), backgroundColor: "#5bc0eb" },
    { label: "Sourdines", data: activity.value.map((a) => a.mutes), backgroundColor: "#fee75c" },
    { label: "Bannissements", data: activity.value.map((a) => a.bans), backgroundColor: "#ed4245" },
  ],
}));

const totalInfractions = computed(() => ({
  w: activity.value.reduce((s, a) => s + a.warns, 0),
  m: activity.value.reduce((s, a) => s + a.mutes, 0),
  b: activity.value.reduce((s, a) => s + a.bans, 0),
}));

const doughnutData = computed(() => ({
  labels: ["Avertissements", "Sourdines", "Bannissements"],
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

const memberGrowthData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Arrivees",
      data: activity.value.map((a) => a.new_members),
      borderColor: "#57f287",
      backgroundColor: "rgba(87, 242, 135, 0.15)",
      fill: true,
      tension: 0.3,
    },
    {
      label: "Departs",
      data: activity.value.map((a) => a.leaves),
      borderColor: "#ed4245",
      backgroundColor: "rgba(237, 66, 69, 0.15)",
      fill: true,
      tension: 0.3,
    },
  ],
}));

const netGrowthData = computed(() => {
  let cumul = 0;
  const data = activity.value.map((a) => {
    cumul += a.new_members - a.leaves;
    return cumul;
  });
  return {
    labels: labels.value,
    datasets: [
      {
        label: "Croissance nette (cumul)",
        data,
        borderColor: "#5865f2",
        backgroundColor: (ctx: ScriptableContext<"line">) =>
          (ctx.raw as number) >= 0 ? "rgba(87, 242, 135, 0.3)" : "rgba(237, 66, 69, 0.3)",
        fill: true,
        tension: 0.3,
      },
    ],
  };
});

const engagementData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Messages / membre actif",
      data: activity.value.map((a) =>
        a.active_members > 0 ? Math.round((a.messages / a.active_members) * 10) / 10 : 0,
      ),
      borderColor: "#e67e22",
      backgroundColor: "rgba(230, 126, 34, 0.15)",
      fill: true,
      tension: 0.3,
    },
  ],
}));

const serverHealthData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Infractions pour 100 messages",
      data: activity.value.map((a) =>
        a.messages > 0 ? Math.round((a.infractions / a.messages) * 10000) / 100 : 0,
      ),
      borderColor: "#e74c3c",
      backgroundColor: "rgba(231, 76, 60, 0.15)",
      fill: true,
      tension: 0.3,
    },
  ],
}));

const topMessagesData = computed(() => ({
  labels: topUsers.value.map((u) => u.username || u.user_id),
  datasets: [
    {
      label: "Messages",
      data: topUsers.value.map((u) => u.message_count),
      backgroundColor: "#5865f2",
      borderRadius: 6,
    },
  ],
}));

const topVoiceData = computed(() => ({
  labels: topUsers.value.map((u) => u.username || u.user_id),
  datasets: [
    {
      label: "Heures vocales",
      data: topUsers.value.map((u) => Math.round(u.voice_hours * 10) / 10),
      backgroundColor: "#57f287",
      borderRadius: 6,
    },
  ],
}));

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
  <section class="dash-section">
    <h2 class="section-title">Graphiques d'activite</h2>

    <div v-if="error" class="error-msg">Erreur chargement graphiques : {{ error }}</div>
    <div v-else-if="!loading && activity.length > 0" class="charts-grid">
      <div class="card chart-card">
        <h3>Messages</h3>
        <div class="chart-container"><Line :data="messagesChartData" :options="chartOptions" /></div>
      </div>
      <div class="card chart-card">
        <h3>Activite vocale</h3>
        <div class="chart-container"><Line :data="voiceChartData" :options="chartOptions" /></div>
      </div>
      <div class="card chart-card">
        <h3>Infractions</h3>
        <div class="chart-container"><Bar :data="infractionsChartData" :options="chartOptions" /></div>
      </div>
      <div class="card chart-card">
        <h3>Repartition des infractions</h3>
        <div class="chart-container chart-container--small">
          <Doughnut :data="doughnutData" :options="doughnutOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Croissance membres</h3>
        <div class="chart-container"><Line :data="memberGrowthData" :options="chartOptions" /></div>
      </div>
      <div class="card chart-card">
        <h3>Croissance nette du serveur</h3>
        <div class="chart-container"><Line :data="netGrowthData" :options="chartOptions" /></div>
      </div>
      <div class="card chart-card">
        <h3>Engagement (messages / membre)</h3>
        <div class="chart-container"><Line :data="engagementData" :options="chartOptions" /></div>
      </div>
      <div class="card chart-card">
        <h3>Sante du serveur</h3>
        <div class="chart-container"><Line :data="serverHealthData" :options="chartOptions" /></div>
      </div>
      <div v-if="topUsers.length > 0" class="card chart-card">
        <h3>Top membres (messages)</h3>
        <div class="chart-container chart-container--tall">
          <Bar :data="topMessagesData" :options="horizontalBarOptions" />
        </div>
      </div>
      <div v-if="topUsers.length > 0" class="card chart-card">
        <h3>Top membres (vocal)</h3>
        <div class="chart-container chart-container--tall">
          <Bar :data="topVoiceData" :options="horizontalBarOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Membres actifs</h3>
        <div class="chart-container"><Line :data="membersChartData" :options="chartOptions" /></div>
      </div>
    </div>
    <div v-else-if="loading" class="loading">Chargement des graphiques...</div>
    <div v-else class="empty">
      Pas encore de donnees d'activite. Les graphiques apparaitront apres quelques heures d'utilisation.
    </div>
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

.charts-grid {
  display: grid;
  /* 3 colonnes sur ecrans larges, 2 colonnes au milieu, 1 sur mobile.
     minmax(0, 1fr) empeche les cells de deborder leur largeur. */
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 16px;
}

@media (max-width: 1300px) {
  .charts-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
  }
}

@media (max-width: 800px) {
  .charts-grid {
    grid-template-columns: 1fr;
  }
}

.chart-card {
  padding: var(--space-xl); /* override .card : plus d'espace pour les graphes */
  min-width: 0; /* empeche l'expansion due au contenu */
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
  height: 220px;
  position: relative;
}

.chart-container--tall { height: 300px; }

.chart-container--small {
  height: 200px;
  max-width: 300px;
  margin: 0 auto;
}

.error-msg {
  color: var(--danger);
  background-color: var(--danger-bg);
  border: 1px solid var(--danger);
  border-radius: 8px;
  padding: 12px 16px;
  font-size: 13px;
}

.loading,
.empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
