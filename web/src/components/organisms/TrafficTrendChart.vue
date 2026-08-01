<script setup lang="ts">
import { errMsg } from "@/utils/errMsg";
import AppSelect from "@/components/atoms/AppSelect.vue";
import { computed, onMounted, ref, watch } from "vue";
import { Line } from "vue-chartjs";
import { registerChartJs } from "@/utils/chartjs";
import { serverSecurityService, type TrafficTrendResponse } from "@/services/serverSecurityService";
import { useToast } from "@/composables/useToast";

registerChartJs();

const { error: showError } = useToast();

const window = ref<"1h" | "6h" | "24h" | "7d">("24h");
const data = ref<TrafficTrendResponse | null>(null);
const loading = ref(false);

async function load() {
  loading.value = true;
  try {
    data.value = await serverSecurityService.trafficTrend(window.value, 5);
  } catch (e) {
    showError(`Trafic : ${errMsg(e)}`);
    data.value = null;
  } finally {
    loading.value = false;
  }
}

onMounted(load);
watch(window, load);

const chartData = computed(() => {
  if (!data.value) return null;
  const labels = data.value.datapoints.map((d) =>
    new Date(d.timestamp).toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" }),
  );
  return {
    labels,
    datasets: [
      {
        label: "Total requêtes",
        data: data.value.datapoints.map((d) => d.total),
        borderColor: "rgba(99, 102, 241, 1)",
        backgroundColor: "rgba(99, 102, 241, 0.15)",
        fill: true,
        tension: 0.3,
      },
      {
        label: "Erreurs (4xx/5xx)",
        data: data.value.datapoints.map((d) => d.errors),
        borderColor: "rgba(239, 68, 68, 1)",
        backgroundColor: "rgba(239, 68, 68, 0.15)",
        fill: true,
        tension: 0.3,
      },
    ],
  };
});

const chartOptions = computed(() => ({
  responsive: true,
  maintainAspectRatio: false,
  plugins: {
    legend: { position: "top" as const, labels: { color: "rgba(255,255,255,0.7)" } },
    tooltip: { mode: "index" as const, intersect: false },
  },
  scales: {
    x: { ticks: { color: "rgba(255,255,255,0.5)", maxTicksLimit: 12 }, grid: { display: false } },
    y: { beginAtZero: true, ticks: { color: "rgba(255,255,255,0.5)" }, grid: { color: "rgba(255,255,255,0.05)" } },
  },
  interaction: { mode: "nearest" as const, axis: "x" as const, intersect: false },
}));
</script>

<template>
  <section class="card">
    <div class="card-head">
      <h2>📈 Trafic anormal</h2>
      <div class="card-actions">
        <AppSelect v-model="window">
          <option value="1h">1h</option>
          <option value="6h">6h</option>
          <option value="24h">24h</option>
          <option value="7d">7j</option>
        </AppSelect>
        <button class="btn xs" @click="load">↻</button>
      </div>
    </div>

    <div v-if="loading" class="muted small">Chargement…</div>

    <div v-else-if="data && data.alert" class="alert-banner">
      ⚠️ <strong>Pic anormal détecté</strong> : {{ data.alert_reason }}
    </div>

    <div v-if="data && data.datapoints.length > 0" class="chart-wrap">
      <Line v-if="chartData" :data="chartData" :options="chartOptions" />
    </div>
    <div v-else-if="data" class="muted small">Pas assez de données pour la période sélectionnée.</div>

    <div v-if="data" class="stats">
      <div class="stat">
        <span class="lbl">Pic</span>
        <strong>{{ data.peak }}</strong> req/bucket
      </div>
      <div class="stat">
        <span class="lbl">Moyenne</span>
        <strong>{{ data.baseline_avg.toFixed(1) }}</strong> req/bucket
      </div>
      <div class="stat">
        <span class="lbl">Buckets</span>
        <strong>{{ data.datapoints.length }}</strong>
      </div>
    </div>
  </section>
</template>

<style scoped>
.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 18px 20px;
  margin-bottom: 16px;
}
.card-head {
  display: flex; justify-content: space-between; align-items: center;
  flex-wrap: wrap; gap: 10px; margin-bottom: 12px;
}
.card-head h2 { margin: 0; font-size: 16px; }
.card-actions { display: flex; gap: 8px; align-items: center; }
.muted { color: var(--text-secondary); }
.muted.small { font-size: 12px; }

.alert-banner {
  background: color-mix(in srgb, var(--danger) 12%, transparent);
  border-left: 3px solid var(--danger);
  border-radius: var(--radius-sm);
  padding: 10px 14px;
  margin-bottom: 12px;
  font-size: 13px;
  color: var(--danger);
}

.chart-wrap { height: 280px; margin: 12px 0; }

.stats {
  display: flex; gap: 24px; padding-top: 10px;
  border-top: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
}
.stat { display: flex; flex-direction: column; }
.stat .lbl { font-size: 11px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.4px; }
.stat strong { font-size: 18px; color: var(--text-primary); }

.btn { padding: 7px 14px; border-radius: var(--radius-md); border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-primary); font-size: 12px; cursor: pointer; }
.btn.xs { padding: 3px 8px; font-size: 11px; }
.btn:hover { border-color: var(--accent); color: var(--accent); }
select { padding: 5px 8px; border-radius: var(--radius-sm); border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-primary); font-size: 12px; }
</style>
