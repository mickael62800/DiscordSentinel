<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from "vue";
import { systemService, type SystemInfo } from "@/services/systemService";
import { useToast } from "@/composables/useToast";
import DockerAdminSection from "@/components/organisms/DockerAdminSection.vue";
import DiskTrendChart from "@/components/organisms/DiskTrendChart.vue";

const { error: showError } = useToast();

const info = ref<SystemInfo | null>(null);
const loading = ref(true);
const refreshing = ref(false);
const autoRefresh = ref(true);
let pollHandle: number | null = null;

async function fetchInfo() {
  refreshing.value = true;
  try {
    info.value = await systemService.getInfo();
  } catch (e) {
    console.error(e);
    showError("Erreur chargement système.");
  } finally {
    loading.value = false;
    refreshing.value = false;
  }
}

function startPolling() {
  if (pollHandle !== null) return;
  pollHandle = window.setInterval(fetchInfo, 120_000);
}
function stopPolling() {
  if (pollHandle !== null) {
    clearInterval(pollHandle);
    pollHandle = null;
  }
}
function toggleAutoRefresh() {
  autoRefresh.value = !autoRefresh.value;
  if (autoRefresh.value) startPolling();
  else stopPolling();
}

onMounted(() => {
  fetchInfo();
  if (autoRefresh.value) startPolling();
});
onUnmounted(stopPolling);

// ── Helpers ──
function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}j ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatGb(gb: number): string {
  if (gb < 1) return `${(gb * 1024).toFixed(0)} MB`;
  return `${gb.toFixed(1)} GB`;
}

function diskBarColor(pct: number): string {
  if (pct >= 90) return "var(--danger)";
  if (pct >= 75) return "var(--warning, #e67e22)";
  return "var(--success, #2ecc71)";
}

const ramPct = computed(() =>
  info.value && info.value.host.mem_total_mb > 0
    ? (info.value.host.mem_used_mb / info.value.host.mem_total_mb) * 100
    : 0,
);
const cpuPct = computed(() => info.value?.host.cpu_percent ?? 0);

const onlineBots = computed(() => info.value?.bots.filter((b) => b.online).length ?? 0);
const totalBots = computed(() => info.value?.bots.length ?? 0);
const onlineWorkers = computed(() => info.value?.workers.filter((w) => w.online).length ?? 0);
const totalWorkers = computed(() => info.value?.workers.length ?? 0);

const allHealthy = computed(() => {
  const h = info.value?.health;
  return h?.api_responding && h?.postgres_responding && h?.redis_responding;
});
</script>

<template>
  <div class="dashboard">
    <div class="dashboard-header">
      <h1>État du serveur</h1>
      <div class="header-actions">
        <button
          class="auto-toggle"
          :class="{ active: autoRefresh }"
          :title="autoRefresh ? 'Auto-refresh actif (2 min)' : 'Auto-refresh désactivé'"
          @click="toggleAutoRefresh"
        >
          <span class="dot" :class="{ pulse: autoRefresh }"></span>
          {{ autoRefresh ? "Live (2 min)" : "Pause" }}
        </button>
        <button
          class="refresh-btn"
          :disabled="refreshing"
          @click="fetchInfo"
        >
          <svg
            :class="['refresh-icon', { spinning: refreshing }]"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
            stroke-linejoin="round"
            width="14"
            height="14"
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

    <div v-if="loading" class="loading">Chargement…</div>

    <template v-else-if="info">
      <!-- ── Bandeau santé global ── -->
      <section class="health-row">
        <div class="health-card" :class="{ ok: info.health.api_responding, ko: !info.health.api_responding }">
          <span class="health-icon">{{ info.health.api_responding ? '✅' : '❌' }}</span>
          <div class="health-text">
            <strong>API HTTP</strong>
            <span class="health-sub">{{ info.health.api_responding ? "Répond" : "Injoignable" }}</span>
          </div>
        </div>
        <div class="health-card" :class="{ ok: info.health.postgres_responding, ko: !info.health.postgres_responding }">
          <span class="health-icon">{{ info.health.postgres_responding ? '✅' : '❌' }}</span>
          <div class="health-text">
            <strong>PostgreSQL</strong>
            <span class="health-sub">{{ info.health.postgres_responding ? `${info.db_size_mb} MB` : "Erreur" }}</span>
          </div>
        </div>
        <div class="health-card" :class="{ ok: info.health.redis_responding, ko: !info.health.redis_responding }">
          <span class="health-icon">{{ info.health.redis_responding ? '✅' : '❌' }}</span>
          <div class="health-text">
            <strong>Redis</strong>
            <span class="health-sub">{{ info.health.redis_responding ? `${info.redis.used_memory_mb} MB` : "Erreur" }}</span>
          </div>
        </div>
        <div class="health-card" :class="{ ok: onlineBots === totalBots && totalBots > 0, ko: onlineBots < totalBots }">
          <span class="health-icon">🤖</span>
          <div class="health-text">
            <strong>Bots</strong>
            <span class="health-sub">{{ onlineBots }} / {{ totalBots }} en ligne</span>
          </div>
        </div>
        <div class="health-card" :class="{ ok: onlineWorkers === totalWorkers && totalWorkers > 0, ko: onlineWorkers < totalWorkers }">
          <span class="health-icon">⚙️</span>
          <div class="health-text">
            <strong>Workers</strong>
            <span class="health-sub">{{ onlineWorkers }} / {{ totalWorkers }} en ligne</span>
          </div>
        </div>
        <div class="health-card" :class="{ ok: allHealthy, ko: !allHealthy }">
          <span class="health-icon">⏱️</span>
          <div class="health-text">
            <strong>Uptime</strong>
            <span class="health-sub">{{ formatUptime(info.uptime_seconds) }}</span>
          </div>
        </div>
      </section>

      <!-- ── CPU / RAM ── -->
      <section class="dash-section">
        <h2 class="section-title">Ressources host</h2>
        <div class="metrics-grid">
          <div class="metric-card">
            <div class="metric-header">
              <span class="metric-label">CPU host</span>
              <span class="metric-value">{{ cpuPct.toFixed(1) }}%</span>
            </div>
            <div class="bar">
              <div class="bar-fill" :style="{ width: `${Math.min(cpuPct, 100)}%`, background: diskBarColor(cpuPct) }"></div>
            </div>
            <div class="metric-sub">{{ info.host.cpu_cores }} cœurs disponibles</div>
          </div>
          <div class="metric-card">
            <div class="metric-header">
              <span class="metric-label">RAM host</span>
              <span class="metric-value">{{ ramPct.toFixed(1) }}%</span>
            </div>
            <div class="bar">
              <div class="bar-fill" :style="{ width: `${Math.min(ramPct, 100)}%`, background: diskBarColor(ramPct) }"></div>
            </div>
            <div class="metric-sub">
              {{ info.host.mem_used_mb.toLocaleString() }} / {{ info.host.mem_total_mb.toLocaleString() }} MB
            </div>
          </div>
          <div class="metric-card">
            <div class="metric-header">
              <span class="metric-label">CPU process API</span>
              <span class="metric-value">{{ info.process.cpu_percent.toFixed(1) }}%</span>
            </div>
            <div class="bar">
              <div class="bar-fill" :style="{ width: `${Math.min(info.process.cpu_percent, 100)}%`, background: 'var(--accent)' }"></div>
            </div>
            <div class="metric-sub">RAM process : {{ info.process.mem_used_mb }} MB</div>
          </div>
          <div class="metric-card">
            <div class="metric-header">
              <span class="metric-label">Redis</span>
              <span class="metric-value">{{ info.redis.used_memory_mb }} MB</span>
            </div>
            <div class="metric-sub">
              {{ info.redis.connected_clients }} clients · {{ info.redis.total_keys.toLocaleString() }} clés
              · uptime {{ formatUptime(info.redis.uptime_seconds) }}
            </div>
          </div>
        </div>
      </section>

      <!-- ── Disques / montages ── -->
      <section v-if="info.disks.length > 0" class="dash-section">
        <h2 class="section-title">Disques & montages</h2>
        <div class="disk-list">
          <div v-for="d in info.disks" :key="`${d.name}-${d.mount_point}`" class="disk-card">
            <div class="disk-header">
              <span class="disk-mount">📂 {{ d.mount_point }}</span>
              <span class="disk-pct" :style="{ color: diskBarColor(d.usage_percent) }">
                {{ d.usage_percent.toFixed(1) }}%
              </span>
            </div>
            <div class="bar">
              <div
                class="bar-fill"
                :style="{ width: `${Math.min(d.usage_percent, 100)}%`, background: diskBarColor(d.usage_percent) }"
              ></div>
            </div>
            <div class="disk-meta">
              <span><strong>{{ formatGb(d.used_gb) }}</strong> utilisé</span>
              <span class="muted">{{ formatGb(d.available_gb) }} libre</span>
              <span class="muted">{{ formatGb(d.total_gb) }} total</span>
              <span class="disk-fs">{{ d.fs_type }}</span>
              <span v-if="d.name" class="disk-name">{{ d.name }}</span>
            </div>
          </div>
        </div>
      </section>

      <!-- ── Bots & Workers ── -->
      <section class="dash-section">
        <h2 class="section-title">Services Discord ({{ totalBots + totalWorkers }})</h2>
        <div class="services-grid">
          <div class="services-col">
            <h3>🤖 Bots ({{ onlineBots }} / {{ totalBots }})</h3>
            <div v-if="info.bots.length === 0" class="muted">Aucun bot enregistré.</div>
            <ul v-else class="services-list">
              <li v-for="b in info.bots" :key="b.name" :class="{ off: !b.online }">
                <span class="status-dot" :class="b.online ? 'on' : 'off'"></span>
                <span class="service-name">{{ b.name }}</span>
                <span class="service-state">{{ b.online ? 'online' : 'offline' }}</span>
              </li>
            </ul>
          </div>
          <div class="services-col">
            <h3>⚙️ Workers ({{ onlineWorkers }} / {{ totalWorkers }})</h3>
            <div v-if="info.workers.length === 0" class="muted">Aucun worker enregistré.</div>
            <ul v-else class="services-list">
              <li v-for="w in info.workers" :key="w.name" :class="{ off: !w.online }">
                <span class="status-dot" :class="w.online ? 'on' : 'off'"></span>
                <span class="service-name">{{ w.name }}</span>
                <span class="service-state">{{ w.online ? 'online' : 'offline' }}</span>
              </li>
            </ul>
          </div>
        </div>
      </section>

      <!-- ── Disque tendance 7j (depuis cron host) ── -->
      <DiskTrendChart />

      <!-- ── Administration Docker (overview, conteneurs, images, volumes, networks, prune) ── -->
      <DockerAdminSection />
    </template>
  </div>
</template>

<style scoped>
/* ── Header ── */
.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
  margin-bottom: 24px;
  padding-bottom: 18px;
  background:
    linear-gradient(to right,
      transparent 0%,
      color-mix(in srgb, var(--accent) 35%, transparent) 30%,
      color-mix(in srgb, var(--accent) 35%, transparent) 70%,
      transparent 100%) bottom / 100% 1px no-repeat;
}
.dashboard-header h1 {
  margin: 0;
  font-size: 1.6rem;
  font-weight: 700;
}
.header-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}
.auto-toggle {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  padding: 7px 14px;
  border-radius: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}
.auto-toggle.active {
  color: var(--success, #2ecc71);
  border-color: color-mix(in srgb, var(--success, #2ecc71) 50%, var(--border));
}
.dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--text-secondary);
}
.dot.pulse {
  background: var(--success, #2ecc71);
  animation: dot-pulse 1.2 min ease-in-out infinite;
}
@keyframes dot-pulse {
  0%, 100% { opacity: 0.5; transform: scale(1); }
  50% { opacity: 1; transform: scale(1.3); }
}
.refresh-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 7px 14px;
  border-radius: 10px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s ease;
}
.refresh-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: var(--accent);
}
.refresh-icon.spinning { animation: spin 0.9s linear infinite; }
@keyframes spin { to { transform: rotate(360deg); } }

/* ── Health row (bandeau status) ── */
.health-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
  margin-bottom: 24px;
}
.health-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  transition: border-color 0.2s ease;
}
.health-card.ok {
  border-color: color-mix(in srgb, var(--success, #2ecc71) 35%, var(--border));
  background: color-mix(in srgb, var(--success, #2ecc71) 6%, var(--bg-card));
}
.health-card.ko {
  border-color: color-mix(in srgb, var(--danger) 50%, var(--border));
  background: color-mix(in srgb, var(--danger) 8%, var(--bg-card));
}
.health-icon { font-size: 20px; flex-shrink: 0; }
.health-text { display: flex; flex-direction: column; min-width: 0; }
.health-text strong { font-size: 13px; }
.health-sub { font-size: 11px; color: var(--text-secondary); }

/* ── Section générique ── */
.dash-section { margin-bottom: 24px; }
.section-title {
  position: relative;
  font-size: 14px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0 0 14px 0;
  padding: 0 0 8px 14px;
  border-bottom: 1px solid var(--border);
}
.section-title::before {
  content: "";
  position: absolute;
  left: 0;
  top: 2px;
  bottom: 14px;
  width: 3px;
  border-radius: 2px;
  background: linear-gradient(to bottom, var(--accent), color-mix(in srgb, var(--accent) 50%, var(--accent-alt, #a855f7)));
}

/* ── CPU / RAM cards ── */
.metrics-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 14px;
}
.metric-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px;
}
.metric-header {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 8px;
}
.metric-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.4px;
}
.metric-value {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
}
.metric-sub {
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 6px;
}

/* ── Bars ── */
.bar {
  height: 8px;
  background: var(--bg-secondary);
  border-radius: 4px;
  overflow: hidden;
  position: relative;
}
.bar-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.4s ease, background 0.3s ease;
}

/* ── Disques ── */
.disk-list {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 14px;
}
.disk-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px;
}
.disk-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.disk-mount {
  font-family: "JetBrains Mono", monospace;
  font-size: 13px;
  font-weight: 600;
}
.disk-pct {
  font-size: 16px;
  font-weight: 700;
}
.disk-meta {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 8px;
  font-size: 11px;
}
.disk-meta .muted { color: var(--text-secondary); }
.disk-fs, .disk-name {
  background: var(--bg-secondary);
  padding: 1px 6px;
  border-radius: 4px;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-secondary);
}

/* ── Bots & Workers ── */
.services-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}
@media (max-width: 720px) {
  .services-grid { grid-template-columns: 1fr; }
}
.services-col {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px;
}
.services-col h3 {
  margin: 0 0 10px;
  font-size: 14px;
  font-weight: 700;
}
.services-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.services-list li {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  font-size: 13px;
}
.services-list li:last-child { border-bottom: none; }
.services-list li.off .service-name { color: var(--text-secondary); }
.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.status-dot.on {
  background: var(--success, #2ecc71);
  box-shadow: 0 0 6px color-mix(in srgb, var(--success, #2ecc71) 60%, transparent);
}
.status-dot.off { background: var(--danger); }
.service-name {
  flex: 1;
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}
.service-state {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.muted { color: var(--text-secondary); font-size: 13px; }

.loading { text-align: center; padding: 40px; color: var(--text-secondary); }
</style>
