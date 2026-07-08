<script setup lang="ts">
import { errMsg } from "@/utils/errMsg";
import { computed, onMounted, ref } from "vue";
import { Line } from "vue-chartjs";
import { registerChartJs } from "@/utils/chartjs";
import { serverSecurityService, type DiskTrendResponse } from "@/services/serverSecurityService";

registerChartJs();

const data = ref<DiskTrendResponse | null>(null);
const loading = ref(false);
const error = ref<string | null>(null);

async function load() {
  loading.value = true;
  error.value = null;
  try {
    data.value = await serverSecurityService.diskTrend();
  } catch (e) {
    error.value = errMsg(e);
    data.value = null;
  } finally {
    loading.value = false;
  }
}

onMounted(load);

// Groupe les points par mount (chaque mount = 1 série)
const chartData = computed(() => {
  if (!data.value) return null;
  const byMount = new Map<string, { ts: string; pct: number }[]>();
  for (const p of data.value.points) {
    if (!byMount.has(p.mount)) byMount.set(p.mount, []);
    byMount.get(p.mount)!.push({ ts: p.timestamp, pct: p.usage_pct });
  }

  // Trie chaque série par timestamp ascendant
  for (const arr of byMount.values()) {
    arr.sort((a, b) => a.ts.localeCompare(b.ts));
  }

  const allTs = Array.from(new Set(data.value.points.map((p) => p.timestamp))).sort();
  const labels = allTs.map((t) =>
    new Date(t).toLocaleString("fr-FR", { day: "2-digit", month: "2-digit", hour: "2-digit" }),
  );

  // Palette de 8 couleurs bien distinctes (indigo / vert / orange / rose /
  // cyan / violet / jaune / rouge). Suffisant pour ~8 disques montes ;
  // au-dela on cycle mais c'est rare en prod (un host avec 8+ disques).
  const colors = [
    { border: "rgba(99, 102, 241, 1)",  bg: "rgba(99, 102, 241, 0.15)" },  // indigo
    { border: "rgba(34, 197, 94, 1)",   bg: "rgba(34, 197, 94, 0.15)" },   // vert
    { border: "rgba(249, 115, 22, 1)",  bg: "rgba(249, 115, 22, 0.15)" },  // orange
    { border: "rgba(236, 72, 153, 1)",  bg: "rgba(236, 72, 153, 0.15)" },  // rose
    { border: "rgba(6, 182, 212, 1)",   bg: "rgba(6, 182, 212, 0.15)" },   // cyan
    { border: "rgba(168, 85, 247, 1)",  bg: "rgba(168, 85, 247, 0.15)" },  // violet
    { border: "rgba(234, 179, 8, 1)",   bg: "rgba(234, 179, 8, 0.15)" },   // jaune
    { border: "rgba(239, 68, 68, 1)",   bg: "rgba(239, 68, 68, 0.15)" },   // rouge
  ];

  const datasets = Array.from(byMount.entries()).map(([mount, points], i) => {
    const map = new Map(points.map((p) => [p.ts, p.pct]));
    return {
      label: mount,
      data: allTs.map((t) => map.get(t) ?? null),
      borderColor: colors[i % colors.length]!.border,
      backgroundColor: colors[i % colors.length]!.bg,
      fill: true,
      tension: 0.3,
      spanGaps: true,
    };
  });

  return { labels, datasets };
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
    y: {
      beginAtZero: true,
      max: 100,
      ticks: {
        color: "rgba(255,255,255,0.5)",
        callback: (v: string | number) => `${v}%`,
      },
      grid: { color: "rgba(255,255,255,0.05)" },
    },
  },
  interaction: { mode: "nearest" as const, axis: "x" as const, intersect: false },
}));
</script>

<template>
  <section class="dash-section">
    <div class="head">
      <h2 class="section-title">📈 Tendance espace disque</h2>
      <button class="btn xs" @click="load">↻</button>
    </div>
    <div v-if="loading" class="muted">Chargement…</div>
    <div v-else-if="error" class="info">
      <p class="small">{{ error }}</p>
      <p class="hint small">
        Setup : <code>sudo bash sentinel-infrastructure/scripts/setup-host-security.sh disk-trend</code>
      </p>
    </div>
    <div v-else-if="data && data.points.length > 0" class="chart-card">
      <p class="muted small">
        Maj {{ new Date(data.updated_at).toLocaleString("fr-FR") }} ·
        Snapshot toutes les heures · Historique conservé 7 jours
      </p>
      <div class="chart-wrap">
        <Line v-if="chartData" :data="chartData" :options="chartOptions" />
      </div>
    </div>
    <div v-else class="empty">Pas encore d'historique. Le cron tourne toutes les heures.</div>
  </section>
</template>

<style scoped>
.dash-section { margin-bottom: 24px; }
.head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 14px; }
.section-title {
  position: relative;
  font-size: 14px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0;
  padding: 0 0 0 14px;
}
.section-title::before {
  content: ""; position: absolute; left: 0; top: 2px; bottom: 2px;
  width: 3px; border-radius: 2px;
  background: linear-gradient(to bottom, var(--accent), color-mix(in srgb, var(--accent) 50%, var(--accent-alt, #a855f7)));
}
.muted { color: var(--text-secondary); font-size: 12px; }
.muted.small { font-size: 11px; }
.small { font-size: 12px; }
.info {
  padding: 14px;
  background: color-mix(in srgb, var(--accent) 6%, var(--bg-secondary));
  border-left: 3px solid var(--accent);
  border-radius: 4px;
}
.hint { font-family: "JetBrains Mono", monospace; }
.empty { padding: 30px; text-align: center; color: var(--text-secondary); font-style: italic; font-size: 12px; }
.chart-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
}
.chart-wrap { height: 280px; margin-top: 10px; }
.btn { padding: 3px 8px; border-radius: 6px; border: 1px solid var(--border); background: var(--bg-secondary); color: var(--text-primary); font-size: 11px; cursor: pointer; }
.btn:hover { border-color: var(--accent); color: var(--accent); }
</style>
