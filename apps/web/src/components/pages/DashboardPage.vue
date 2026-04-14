<script setup lang="ts">
import { ref, computed } from "vue";
import DashboardStatsSection from "../organisms/DashboardStatsSection.vue";
import DashboardChartsSection from "../organisms/DashboardChartsSection.vue";
import AnalyticsSection from "../organisms/AnalyticsSection.vue";
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

const days = ref(30);

const statsRef = ref<InstanceType<typeof DashboardStatsSection> | null>(null);
const chartsRef = ref<InstanceType<typeof DashboardChartsSection> | null>(null);
const analyticsRef = ref<InstanceType<typeof AnalyticsSection> | null>(null);

const refreshing = ref(false);

async function handleRefresh() {
  refreshing.value = true;
  try {
    await Promise.all([
      statsRef.value?.refresh(),
      chartsRef.value?.refresh(),
      analyticsRef.value?.refresh(),
    ]);
  } finally {
    refreshing.value = false;
  }
}

const periods = computed(() => [7, 14, 30, 90]);
</script>

<template>
  <div class="dashboard">
    <div class="dashboard-header">
      <h1>Tableau de bord</h1>
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
          :title="refreshing ? 'Actualisation en cours…' : 'Actualiser les donnees'"
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

    <DashboardStatsSection ref="statsRef" />
    <DashboardChartsSection ref="chartsRef" :days="days" />
    <AnalyticsSection ref="analyticsRef" :days="days" />
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

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
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

.refresh-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 14px;
  border-radius: 8px;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.15s, background-color 0.15s;
}

.refresh-btn:hover:not(:disabled) {
  color: var(--text-primary);
  background-color: var(--bg-hover);
}

.refresh-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.refresh-icon {
  width: 14px;
  height: 14px;
}

.refresh-icon.spinning {
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  from { transform: rotate(0deg); }
  to { transform: rotate(360deg); }
}
</style>
