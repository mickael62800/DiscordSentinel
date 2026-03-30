<script setup lang="ts">
import { computed } from "vue";
import type { EpochRecord, DatasetInfo } from "../../composables/useAiTraining";
import { Line, Doughnut } from "vue-chartjs";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
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
  ArcElement,
  Title,
  Tooltip,
  Legend,
  Filler,
);

const props = defineProps<{
  epochHistory: EpochRecord[];
  dataset: DatasetInfo | undefined;
}>();

// ── Options graphiques ──
const lineChartOptions = {
  responsive: true,
  maintainAspectRatio: false,
  animation: { duration: 300 },
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

const accuracyChartOptions = {
  ...lineChartOptions,
  scales: {
    ...lineChartOptions.scales,
    y: {
      ...lineChartOptions.scales.y,
      max: 1,
      ticks: {
        ...lineChartOptions.scales.y.ticks,
        callback: (v: number | string) => `${(Number(v) * 100).toFixed(0)}%`,
      },
    },
  },
};

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

const epochLabels = computed(() =>
  props.epochHistory.map((e) => `Epoch ${e.epoch}`)
);

const lossChartData = computed(() => ({
  labels: epochLabels.value,
  datasets: [
    {
      label: "Loss (train)",
      data: props.epochHistory.map((e) => e.loss),
      borderColor: "#ef4444",
      backgroundColor: "rgba(239, 68, 68, 0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 3,
    },
    {
      label: "Loss (validation)",
      data: props.epochHistory.map((e) => e.val_loss),
      borderColor: "#f97316",
      backgroundColor: "rgba(249, 115, 22, 0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 3,
      borderDash: [5, 5],
    },
  ],
}));

const accuracyChartData = computed(() => ({
  labels: epochLabels.value,
  datasets: [
    {
      label: "Accuracy (train)",
      data: props.epochHistory.map((e) => e.accuracy),
      borderColor: "#22c55e",
      backgroundColor: "rgba(34, 197, 94, 0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 3,
    },
    {
      label: "Accuracy (validation)",
      data: props.epochHistory.map((e) => e.val_accuracy),
      borderColor: "#3b82f6",
      backgroundColor: "rgba(59, 130, 246, 0.1)",
      fill: true,
      tension: 0.3,
      pointRadius: 3,
      borderDash: [5, 5],
    },
  ],
}));

const LABEL_COLORS = [
  "#7c3aed", "#5865f2", "#22c55e", "#f97316", "#ef4444",
  "#06b6d4", "#ec4899", "#eab308", "#8b5cf6", "#14b8a6",
];

const datasetChartData = computed(() => {
  const ds = props.dataset;
  if (!ds) return { labels: [], datasets: [] };
  const labels = Object.keys(ds.label_distribution);
  const data = Object.values(ds.label_distribution);
  return {
    labels,
    datasets: [
      {
        data,
        backgroundColor: labels.map((_, i) => LABEL_COLORS[i % LABEL_COLORS.length]),
        borderWidth: 0,
      },
    ],
  };
});

const hasEpochData = computed(() => props.epochHistory.length > 0);
</script>

<template>
  <!-- Distribution du dataset -->
  <section v-if="dataset && Object.keys(dataset.label_distribution).length > 0" class="section-card">
    <h3>Distribution des labels</h3>
    <div class="chart-center">
      <div class="doughnut-wrapper">
        <Doughnut :data="datasetChartData" :options="doughnutOptions" />
      </div>
    </div>
  </section>

  <!-- Courbe Loss -->
  <section v-if="hasEpochData" class="section-card">
    <h3>Loss</h3>
    <div class="chart-wrapper">
      <Line :data="lossChartData" :options="lineChartOptions" />
    </div>
  </section>

  <!-- Courbe Accuracy -->
  <section v-if="hasEpochData" class="section-card">
    <h3>Accuracy</h3>
    <div class="chart-wrapper">
      <Line :data="accuracyChartData" :options="accuracyChartOptions" />
    </div>
  </section>

  <!-- Etat vide quand pas de graphiques -->
  <section v-if="!hasEpochData && !(dataset && Object.keys(dataset.label_distribution).length > 0)" class="section-card charts-empty">
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="empty-icon">
      <polyline points="22 12 18 12 15 21 9 3 6 12 2 12" />
    </svg>
    <p>Les graphiques apparaitront ici une fois un dataset charge ou un entrainement lance.</p>
  </section>
</template>

<style scoped>
.section-card {
  background: var(--bg-secondary, #1e1e2e);
  border-radius: 12px;
  padding: 1.5rem;
  margin-bottom: 1.5rem;
}

.section-card h3 {
  margin: 0 0 1rem;
  font-size: 1rem;
}

/* Graphiques */
.chart-wrapper {
  position: relative;
  height: 200px;
}

.chart-center {
  display: flex;
  justify-content: center;
}

.doughnut-wrapper {
  position: relative;
  width: 100%;
  height: 260px;
}

.charts-empty {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 12px;
  padding: 3rem 1.5rem;
  text-align: center;
  color: var(--text-secondary, #888);
}

.charts-empty p {
  font-size: 0.85rem;
  line-height: 1.5;
}

.empty-icon {
  width: 40px;
  height: 40px;
  opacity: 0.3;
}
</style>
