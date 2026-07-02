<script setup lang="ts">
import { computed, toRef } from "vue";
import { useDashboardCharts } from "@/composables/useDashboardCharts";
import { useAnalytics } from "@/composables/useAnalytics";
import { Line, Bar } from "vue-chartjs";
import { registerChartJs } from "@/utils/chartjs";
import {
  makeLineOptions,
  makeBarOptions,
  colorAt,
  fillColor,
  severityColors,
} from "@/utils/chartTheme";

registerChartJs();

const props = defineProps<{ days: number }>();
const daysRef = toRef(props, "days");

const sharedDays = computed({
  get: () => daysRef.value,
  set: () => {},
});

const { activity, topUsers, loading, error, fetchAll } = useDashboardCharts(sharedDays);
const { analytics, fetchAnalytics } = useAnalytics(sharedDays);

defineExpose({
  refresh: async () => {
    await Promise.all([fetchAll(), fetchAnalytics()]);
  },
});

// Heatmap d'activite (messages) par jour-de-semaine x heure.
// Recuperee de l'ancienne section Analytics : c'est la "partie active" qui
// montre QUAND les membres parlent — utile pour caler des animations.
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

// Doughnut "Repartition des infractions" - re-ajoute a la demande de
// l'utilisateur (le camembert lui plait).
const totalInfractions = computed(() => ({
  w: activity.value.reduce((s, a) => s + a.warns, 0),
  m: activity.value.reduce((s, a) => s + a.mutes, 0),
  b: activity.value.reduce((s, a) => s + a.bans, 0),
}));

const infractionsBarData = computed(() => ({
  labels: ["Avertissements", "Sourdines", "Bannissements"],
  datasets: [
    {
      label: "Total",
      data: [totalInfractions.value.w, totalInfractions.value.m, totalInfractions.value.b],
      backgroundColor: [severityColors.info, severityColors.medium, severityColors.critical],
      borderRadius: 6,
    },
  ],
}));

const infractionsBarOptions = makeBarOptions({}, true);

const chartOptions = makeLineOptions();

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
      borderColor: colorAt(0),
      backgroundColor: fillColor(colorAt(0)),
      fill: true,
      tension: 0.3,
    },
  ],
}));

const voiceChartData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Heures vocales / jour",
      data: activity.value.map((a) => Math.round((a.voice_minutes / 60) * 10) / 10),
      backgroundColor: colorAt(1),
      borderRadius: 4,
    },
  ],
}));

const memberGrowthData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Arrivees",
      data: activity.value.map((a) => a.new_members),
      borderColor: severityColors.low,
      backgroundColor: fillColor(severityColors.low),
      fill: true,
      tension: 0.3,
    },
    {
      label: "Departs",
      data: activity.value.map((a) => a.leaves),
      borderColor: severityColors.critical,
      backgroundColor: fillColor(severityColors.critical),
      fill: true,
      tension: 0.3,
    },
  ],
}));

const engagementData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Messages / membre actif",
      data: activity.value.map((a) =>
        a.active_members > 0 ? Math.round((a.messages / a.active_members) * 10) / 10 : 0,
      ),
      borderColor: colorAt(2),
      backgroundColor: fillColor(colorAt(2)),
      fill: true,
      tension: 0.3,
    },
  ],
}));

// Top 5 messages (l'API renvoie 10, on coupe localement).
const topMessageUsers = computed(() => topUsers.value.slice(0, 5));

const topMessagesData = computed(() => ({
  labels: topMessageUsers.value.map((u) => u.username || u.user_id),
  datasets: [
    {
      label: "Messages",
      data: topMessageUsers.value.map((u) => u.message_count),
      backgroundColor: colorAt(0),
      borderRadius: 6,
    },
  ],
}));

// Le top voice est un classement DIFFERENT du top messages : on re-trie
// localement par voice_hours desc et on filtre les users a 0h, puis on
// coupe au top 5.
const topVoiceUsers = computed(() =>
  [...topUsers.value]
    .filter((u) => u.voice_hours > 0)
    .sort((a, b) => b.voice_hours - a.voice_hours)
    .slice(0, 5),
);

const topVoiceData = computed(() => ({
  labels: topVoiceUsers.value.map((u) => u.username || u.user_id),
  datasets: [
    {
      label: "Heures vocales",
      data: topVoiceUsers.value.map((u) => Math.round(u.voice_hours * 10) / 10),
      backgroundColor: severityColors.low,
      borderRadius: 6,
    },
  ],
}));

const horizontalBarOptions = makeBarOptions({}, true);

const membersChartData = computed(() => ({
  labels: labels.value,
  datasets: [
    {
      label: "Membres actifs",
      data: activity.value.map((a) => a.active_members),
      borderColor: colorAt(6),
      backgroundColor: fillColor(colorAt(6)),
      fill: true,
      tension: 0.3,
    },
  ],
}));
</script>

<template>
  <section class="dash-section">
    <h2 class="section-title">Activite du serveur</h2>

    <div v-if="error" class="error-msg">Erreur chargement graphiques : {{ error }}</div>
    <div v-else-if="!loading && activity.length > 0" class="charts-grid">
      <div class="card chart-card">
        <h3>Messages</h3>
        <div class="chart-container"><Line :data="messagesChartData" :options="chartOptions" /></div>
      </div>
      <div class="card chart-card">
        <h3>Activite vocale</h3>
        <div class="chart-container"><Bar :data="voiceChartData" :options="chartOptions" /></div>
      </div>
      <div class="card chart-card">
        <h3>Croissance membres</h3>
        <div class="chart-container"><Line :data="memberGrowthData" :options="chartOptions" /></div>
      </div>
      <div class="card chart-card">
        <h3>Engagement (messages / membre)</h3>
        <div class="chart-container"><Line :data="engagementData" :options="chartOptions" /></div>
      </div>
      <div v-if="topMessageUsers.length > 0" class="card chart-card">
        <h3>Top 5 membres (messages)</h3>
        <div class="chart-container">
          <Bar :data="topMessagesData" :options="horizontalBarOptions" />
        </div>
      </div>
      <div v-if="topVoiceUsers.length > 0" class="card chart-card">
        <h3>Top 5 membres (vocal)</h3>
        <div class="chart-container">
          <Bar :data="topVoiceData" :options="horizontalBarOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Membres actifs</h3>
        <div class="chart-container"><Line :data="membersChartData" :options="chartOptions" /></div>
      </div>
      <div class="card chart-card">
        <h3>Repartition des infractions</h3>
        <div class="chart-container">
          <Bar :data="infractionsBarData" :options="infractionsBarOptions" />
        </div>
      </div>
      <div class="card chart-card">
        <h3>Heatmap activite (messages par heure)</h3>
        <div v-if="heatmapGrid" class="heatmap-wrapper">
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
        <div v-else class="heatmap-empty">
          Pas encore assez de donnees pour afficher la heatmap.
        </div>
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
/* Petite barre verticale accent à gauche du titre — touche cosy. */
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

.charts-grid {
  display: grid;
  /* 3 colonnes sur ecrans larges, 2 colonnes au milieu, 1 sur mobile.
     minmax(0, 1fr) empeche les cells de deborder leur largeur. */
  grid-template-columns: repeat(3, minmax(0, 1fr));
  gap: 20px;
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
  position: relative;
  border: 1px solid var(--border);
  border-radius: 12px;
  background: var(--bg-card);
  /* Stagger entrance + transition cosy au hover. */
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

/* Stagger : 9 cartes apparaissent en cascade. */
.chart-card:nth-child(1) { animation-delay: 0.05s; }
.chart-card:nth-child(2) { animation-delay: 0.10s; }
.chart-card:nth-child(3) { animation-delay: 0.15s; }
.chart-card:nth-child(4) { animation-delay: 0.20s; }
.chart-card:nth-child(5) { animation-delay: 0.25s; }
.chart-card:nth-child(6) { animation-delay: 0.30s; }
.chart-card:nth-child(7) { animation-delay: 0.35s; }
.chart-card:nth-child(8) { animation-delay: 0.40s; }
.chart-card:nth-child(n+9) { animation-delay: 0.45s; }

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

.chart-container--tall { height: 320px; }

.chart-container--small {
  height: 200px;
  max-width: 300px;
  margin: 0 auto;
}

.heatmap-wrapper { width: 100%; }
.heatmap-table {
  border-collapse: separate;
  border-spacing: 2px;
  width: 100%;
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
.heatmap-empty {
  padding: 24px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
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

@media (prefers-reduced-motion: reduce) {
  .chart-card {
    animation: none !important;
    opacity: 1;
    transform: none !important;
  }
  .chart-card:hover { transform: none; }
}
</style>
