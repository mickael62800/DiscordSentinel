<script setup lang="ts">
import { ref, computed } from "vue";
import DashboardChartsSection from "../organisms/DashboardChartsSection.vue";
import { registerChartJs } from "@/utils/chartjs";

registerChartJs();

const days = ref(30);

const chartsRef = ref<InstanceType<typeof DashboardChartsSection> | null>(null);

const refreshing = ref(false);

async function handleRefresh() {
  refreshing.value = true;
  try {
    await chartsRef.value?.refresh();
  } finally {
    refreshing.value = false;
  }
}

const periods = computed(() => [7, 14, 30, 90]);
</script>

<template>
  <div class="dashboard page--wide">
    <div class="dashboard-header">
      <h1>Statistiques du serveur</h1>
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

    <DashboardChartsSection ref="chartsRef" :days="days" />
  </div>
</template>

<style scoped>
.dashboard-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 16px;
  flex-wrap: wrap;
  margin-bottom: 24px;
  padding-bottom: 18px;
  /* Bordure douce dégradée sous le header pour structurer la page. */
  border-bottom: 1px solid transparent;
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
  /* Gradient text discret : juste une touche au mot, pas trop flashy. */
  background: linear-gradient(
    90deg,
    var(--text-primary) 0%,
    color-mix(in srgb, var(--accent) 60%, var(--text-primary)) 50%,
    var(--text-primary) 100%
  );
  background-size: 200% auto;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  color: transparent;
  animation: stats-title-shimmer 10s linear infinite;
  letter-spacing: 0.3px;
}
@keyframes stats-title-shimmer {
  0%   { background-position: 200% center; }
  100% { background-position: -200% center; }
}

.header-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

/* ── Période selector — segmented control style cosy ─────── */
.period-selector {
  display: flex;
  gap: 2px;
  background-color: color-mix(in srgb, var(--bg-card) 80%, transparent);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 3px;
  position: relative;
  box-shadow:
    inset 0 1px 2px rgba(0, 0, 0, 0.18),
    0 1px 0 color-mix(in srgb, white 6%, transparent);
}

.period-btn {
  position: relative;
  padding: 6px 14px;
  border-radius: 7px;
  background: none;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease,
    background 0.25s ease,
    box-shadow 0.25s ease;
}

/* Indicateur subtil sous chaque bouton inactif au hover (souligné court). */
.period-btn::after {
  content: "";
  position: absolute;
  left: 50%;
  bottom: 3px;
  width: 0;
  height: 2px;
  border-radius: 2px;
  background: var(--accent);
  transform: translateX(-50%);
  transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.period-btn:hover:not(.active) {
  color: var(--text-primary);
  background-color: color-mix(in srgb, var(--accent) 8%, transparent);
}
.period-btn:hover:not(.active)::after {
  width: 60%;
}

.period-btn.active {
  /* Gradient discret + double shadow (interne lumineuse + externe douce)
     pour donner un effet "embouti / glossy" sans être agressif. */
  background: linear-gradient(135deg,
    var(--accent),
    color-mix(in srgb, var(--accent) 75%, var(--accent-alt, #a855f7)));
  color: white;
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 35%, transparent),
    inset 0 -1px 0 color-mix(in srgb, black 15%, transparent),
    0 2px 8px color-mix(in srgb, var(--accent) 30%, transparent);
  text-shadow: 0 1px 1px rgba(0, 0, 0, 0.12);
}

.period-btn:active {
  /* Petit enfoncement tactile au clic. */
  transform: scale(0.96);
  transition-duration: 0.08s;
}

/* ── Refresh button — propre, tactile, accent au hover ─────── */
.refresh-btn {
  display: inline-flex;
  align-items: center;
  gap: 7px;
  padding: 7px 14px;
  border-radius: 10px;
  background:
    linear-gradient(180deg,
      color-mix(in srgb, white 4%, var(--bg-card)),
      var(--bg-card));
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease,
    background 0.25s ease,
    border-color 0.2s ease,
    box-shadow 0.25s ease;
  /* Inner highlight subtil pour la sensation "embouti". */
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 6%, transparent);
}

.refresh-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--accent) 10%, var(--bg-card)),
    color-mix(in srgb, var(--accent) 6%, var(--bg-card)));
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 10%, transparent),
    0 4px 12px color-mix(in srgb, var(--accent) 18%, transparent);
}

/* L'icône fait un demi-tour d'aperçu au hover (preview du refresh). */
.refresh-btn:hover:not(:disabled) .refresh-icon:not(.spinning) {
  transform: rotate(180deg);
}
.refresh-icon {
  transition: transform 0.45s cubic-bezier(0.4, 0, 0.2, 1);
}

.refresh-btn:active:not(:disabled) {
  transform: scale(0.97);
  transition-duration: 0.08s;
}

.refresh-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

@media (prefers-reduced-motion: reduce) {
  .dashboard-header h1 {
    animation: none;
    background: none;
    -webkit-text-fill-color: var(--text-primary);
    color: var(--text-primary);
  }
  .period-btn,
  .period-btn:hover,
  .period-btn:active,
  .refresh-btn,
  .refresh-btn:hover,
  .refresh-btn:active { transform: none; }
  .refresh-icon { transition: none !important; }
  .period-btn::after { transition: none !important; }
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
