<script setup lang="ts">
import { computed } from "vue";
import { useSystemInfo } from "@/composables/useSystemInfo";

const { info, loading, error, fetchInfo } = useSystemInfo();

defineExpose({ refresh: fetchInfo });

const memPct = computed(() => {
  if (!info.value || info.value.host.mem_total_mb === 0) return 0;
  return Math.round((info.value.host.mem_used_mb / info.value.host.mem_total_mb) * 100);
});

function formatUptime(sec: number): string {
  const d = Math.floor(sec / 86400);
  const h = Math.floor((sec % 86400) / 3600);
  const m = Math.floor((sec % 3600) / 60);
  if (d > 0) return `${d}j ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatMemory(mb: number): string {
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} Go`;
  return `${mb} Mo`;
}
</script>

<template>
  <section class="dash-section">
    <h2 class="section-title">Detail du systeme</h2>

    <div v-if="loading && !info" class="loading">Chargement…</div>
    <div v-else-if="error" class="error-msg">Erreur : {{ error }}</div>
    <div v-else-if="info" class="detail-grid">
      <!-- Services (bots + workers combines) -->
      <div class="card detail-card services-card">
        <div class="card-header">
          <h3>Services</h3>
          <span class="count-pill">
            {{ info.bots.filter(b => b.online).length + info.workers.filter(w => w.online).length }}
            / {{ info.bots.length + info.workers.length }}
          </span>
        </div>

        <!-- Sous-section Bots -->
        <div class="subsection">
          <div class="subsection-header">
            <h4>Bots</h4>
            <span class="count-pill small">
              {{ info.bots.filter(b => b.online).length }} / {{ info.bots.length }}
            </span>
          </div>
          <div v-if="info.bots.length === 0" class="empty-list">
            Aucun bot enregistre.
          </div>
          <div v-else class="service-list">
            <div
              v-for="b in info.bots"
              :key="b.name"
              :class="['service-row', b.online ? 'online' : 'offline']"
            >
              <span class="service-dot" />
              <span class="service-name">{{ b.name }}</span>
              <span class="service-status">{{ b.online ? "En ligne" : "Hors ligne" }}</span>
            </div>
          </div>
        </div>

        <!-- Sous-section Workers -->
        <div class="subsection">
          <div class="subsection-header">
            <h4>Workers</h4>
            <span class="count-pill small">
              {{ info.workers.filter(w => w.online).length }} / {{ info.workers.length }}
            </span>
          </div>
          <div v-if="info.workers.length === 0" class="empty-list">
            Aucun worker enregistre.
          </div>
          <div v-else class="service-list">
            <div
              v-for="w in info.workers"
              :key="w.name"
              :class="['service-row', w.online ? 'online' : 'offline']"
            >
              <span class="service-dot" />
              <span class="service-name">{{ w.name }}</span>
              <span class="service-status">{{ w.online ? "En ligne" : "Hors ligne" }}</span>
            </div>
          </div>
        </div>
      </div>

      <!-- Ressources systeme (host) -->
      <div class="card detail-card">
        <div class="card-header">
          <h3>Ressources</h3>
        </div>
        <div class="metrics-list">
          <div class="metric">
            <div class="metric-header">
              <span class="metric-label">CPU ({{ info.host.cpu_cores }} coeurs)</span>
              <span class="metric-value">{{ info.host.cpu_percent.toFixed(1) }}%</span>
            </div>
            <div class="metric-bar">
              <div
                class="metric-bar-fill cpu"
                :style="{ width: `${Math.min(info.host.cpu_percent, 100)}%` }"
              />
            </div>
          </div>

          <div class="metric">
            <div class="metric-header">
              <span class="metric-label">Memoire</span>
              <span class="metric-value">
                {{ formatMemory(info.host.mem_used_mb) }} / {{ formatMemory(info.host.mem_total_mb) }}
                ({{ memPct }}%)
              </span>
            </div>
            <div class="metric-bar">
              <div
                class="metric-bar-fill mem"
                :style="{ width: `${memPct}%` }"
              />
            </div>
          </div>

          <div class="metric-row">
            <span class="metric-label">CPU process API</span>
            <span class="metric-value">{{ info.process.cpu_percent.toFixed(1) }}%</span>
          </div>
          <div class="metric-row">
            <span class="metric-label">RAM process API</span>
            <span class="metric-value">{{ formatMemory(info.process.mem_used_mb) }}</span>
          </div>
          <div class="metric-row">
            <span class="metric-label">Uptime API</span>
            <span class="metric-value">{{ formatUptime(info.uptime_seconds) }}</span>
          </div>
          <div class="metric-row">
            <span class="metric-label">Taille BDD</span>
            <span class="metric-value">{{ formatMemory(info.db_size_mb) }}</span>
          </div>
          <div class="metric-row">
            <span class="metric-label">Redis memoire</span>
            <span class="metric-value">{{ formatMemory(info.redis.used_memory_mb) }}</span>
          </div>
          <div class="metric-row">
            <span class="metric-label">Redis cles</span>
            <span class="metric-value">{{ info.redis.total_keys.toLocaleString() }}</span>
          </div>
          <div class="metric-row">
            <span class="metric-label">Redis clients</span>
            <span class="metric-value">{{ info.redis.connected_clients }}</span>
          </div>
          <div class="metric-row">
            <span class="metric-label">Redis uptime</span>
            <span class="metric-value">{{ formatUptime(info.redis.uptime_seconds) }}</span>
          </div>
        </div>
      </div>
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

.detail-grid {
  display: grid;
  /* 2 colonnes au-dela de ~900px, 1 en dessous.
     Services (bots + workers) a gauche, Ressources a droite. */
  grid-template-columns: repeat(auto-fit, minmax(380px, 1fr));
  gap: 16px;
}

.services-card .subsection + .subsection {
  margin-top: 18px;
  padding-top: 14px;
  border-top: 1px dashed var(--border);
}

.subsection-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 10px;
}

.subsection-header h4 {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.4px;
  margin: 0;
}

.count-pill.small {
  font-size: 10px;
  padding: 2px 8px;
}

.detail-card {
  padding: 18px 20px; /* override .card (var(--space-lg)=16px) pour mieux aligner les metriques */
  min-width: 0;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 14px;
}

.card-header h3 {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
  margin: 0;
}

.count-pill {
  font-size: 11px;
  font-weight: 600;
  padding: 3px 10px;
  border-radius: var(--radius-pill);
  background-color: var(--accent-bg);
  color: var(--accent);
}

.service-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.empty-list {
  padding: 20px 10px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 12px;
  font-style: italic;
}

.service-row {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 6px 10px;
  border-radius: 6px;
  font-size: 12px;
  transition: background-color var(--transition-fast);
}

.service-row:hover {
  background-color: var(--bg-hover);
}

.service-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}

.service-row.online .service-dot {
  background-color: var(--success);
  box-shadow: 0 0 6px color-mix(in srgb, var(--success) 60%, transparent);
}

.service-row.offline .service-dot {
  background-color: var(--danger);
}

.service-name {
  flex: 1;
  font-family: "Courier New", monospace;
  color: var(--text-primary);
}

.service-row.offline .service-name {
  color: var(--text-secondary);
  opacity: 0.7;
}

.service-status {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.service-row.online .service-status {
  color: var(--success);
}

.service-row.offline .service-status {
  color: #ed4245;
}

.metrics-list {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.metric {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.metric-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
}

.metric-label {
  color: var(--text-secondary);
  font-weight: 600;
}

.metric-value {
  color: var(--text-primary);
  font-weight: 600;
  font-family: "Courier New", monospace;
}

.metric-bar {
  height: 8px;
  background-color: var(--bg-hover);
  border-radius: 4px;
  overflow: hidden;
}

.metric-bar-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.3s ease;
}

.metric-bar-fill.cpu {
  background: linear-gradient(90deg, var(--accent), var(--accent-alt));
}

.metric-bar-fill.mem {
  background: linear-gradient(90deg, var(--success), #2ecc71);
}

.metric-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  font-size: 12px;
  padding-top: 10px;
  border-top: 1px dashed var(--border);
}

.loading,
.error-msg {
  padding: 30px;
  text-align: center;
  font-size: 13px;
}

.loading {
  color: var(--text-secondary);
}

.error-msg {
  color: var(--danger);
}
</style>
