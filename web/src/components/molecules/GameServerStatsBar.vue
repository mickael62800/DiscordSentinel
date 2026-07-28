<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { errMsg } from "@/utils/errMsg";
import {
  gamePortalService,
  type GameServerStats,
} from "@/services/gamePortalService";

const props = defineProps<{
  serverId: string | null;
  /** Si false, on ne poll pas (ex: serveur stopped). */
  active: boolean;
  /** Cadence de poll en ms. Defaut 5s. */
  intervalMs?: number;
}>();

const stats = ref<GameServerStats | null>(null);
const error = ref<string | null>(null);
let timer: number | undefined;

const memPercent = computed(() => {
  if (!stats.value || stats.value.memory_limit_mb === 0) return 0;
  return Math.min(
    100,
    (stats.value.memory_used_mb / stats.value.memory_limit_mb) * 100,
  );
});

const cpuPercent = computed(() =>
  stats.value ? Math.min(100, Math.max(0, stats.value.cpu_percent)) : 0,
);

const memColor = computed(() => {
  const p = memPercent.value;
  if (p > 90) return "var(--danger)";
  if (p > 75) return "var(--warning, #fee75c)";
  return "var(--accent-alt, #7c5cfc)";
});

const cpuColor = computed(() => {
  const p = cpuPercent.value;
  if (p > 90) return "var(--danger)";
  if (p > 75) return "var(--warning, #fee75c)";
  return "var(--accent)";
});

async function fetchStats() {
  if (!props.serverId || !props.active) return;
  try {
    stats.value = await gamePortalService.getStats(props.serverId);
    error.value = null;
  } catch (e) {
    error.value = errMsg(e);
  }
}

function start() {
  stop();
  if (!props.serverId || !props.active) return;
  void fetchStats();
  timer = window.setInterval(fetchStats, props.intervalMs ?? 5000);
}

function stop() {
  if (timer) {
    window.clearInterval(timer);
    timer = undefined;
  }
}

onMounted(start);
onUnmounted(stop);
watch(() => [props.serverId, props.active], start);

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  if (n < 1024 * 1024 * 1024) return `${(n / 1024 / 1024).toFixed(1)} MB`;
  return `${(n / 1024 / 1024 / 1024).toFixed(2)} GB`;
}
</script>

<template>
  <div class="stats-bar">
    <div v-if="!active" class="stats-empty">Serveur arrêté</div>
    <div v-else-if="error" class="stats-empty err">⚠ {{ error }}</div>
    <div v-else-if="!stats" class="stats-empty">Chargement…</div>
    <template v-else>
      <div class="metric">
        <div class="metric-head">
          <span class="metric-label">CPU</span>
          <span class="metric-value">{{ cpuPercent.toFixed(1) }}%</span>
        </div>
        <div class="bar">
          <span :style="{ width: cpuPercent + '%', background: cpuColor }" />
        </div>
      </div>

      <div class="metric">
        <div class="metric-head">
          <span class="metric-label">RAM</span>
          <span class="metric-value">
            {{ stats.memory_used_mb.toLocaleString() }} /
            {{ stats.memory_limit_mb.toLocaleString() }} MB
            <span class="muted">({{ memPercent.toFixed(0) }}%)</span>
          </span>
        </div>
        <div class="bar">
          <span :style="{ width: memPercent + '%', background: memColor }" />
        </div>
      </div>

      <div class="net">
        <span class="net-item">↓ {{ formatBytes(stats.network_rx_bytes) }}</span>
        <span class="net-item">↑ {{ formatBytes(stats.network_tx_bytes) }}</span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.stats-bar {
  display: flex;
  flex-direction: column;
  gap: 10px;
  padding: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md, 8px);
}

.stats-empty {
  color: var(--text-secondary);
  font-size: 12px;
  text-align: center;
  padding: 8px;
}

.stats-empty.err {
  color: var(--danger);
}

.metric {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.metric-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 11px;
  color: var(--text-secondary);
}

.metric-label {
  text-transform: uppercase;
  letter-spacing: 0.04em;
  font-weight: 600;
}

.metric-value {
  font-family: monospace;
  color: var(--text-primary);
}

.muted {
  color: var(--text-secondary);
}

.bar {
  height: 6px;
  background: var(--bg-primary);
  border-radius: 3px;
  overflow: hidden;
}

.bar span {
  display: block;
  height: 100%;
  background: var(--accent);
  transition: width 0.6s ease, background 0.3s ease;
}

.net {
  display: flex;
  gap: 14px;
  font-size: 11px;
  color: var(--text-secondary);
  font-family: monospace;
}

.net-item {
  display: inline-flex;
  gap: 4px;
}
</style>
