<script setup lang="ts">
import { computed } from "vue";
import { useSecurity } from "@/composables/useSecurity";

const { events } = useSecurity();

const stats = computed(() => {
  const list = events.value;
  return {
    total: list.length,
    critical: list.filter((e) => e.severity === "critical").length,
    high: list.filter((e) => e.severity === "high").length,
    medium: list.filter((e) => e.severity === "medium").length,
    low: list.filter((e) => e.severity === "low").length,
  };
});
</script>

<template>
  <div class="stats-grid">
    <div class="card stat-card stat-total">
      <span class="stat-label">Total</span>
      <span class="stat-value">{{ stats.total }}</span>
    </div>
    <div class="card stat-card stat-critical">
      <span class="stat-label">Critiques</span>
      <span class="stat-value">{{ stats.critical }}</span>
    </div>
    <div class="card stat-card stat-high">
      <span class="stat-label">Eleves</span>
      <span class="stat-value">{{ stats.high }}</span>
    </div>
    <div class="card stat-card stat-medium">
      <span class="stat-label">Moyens</span>
      <span class="stat-value">{{ stats.medium }}</span>
    </div>
    <div class="card stat-card stat-low">
      <span class="stat-label">Faibles</span>
      <span class="stat-value">{{ stats.low }}</span>
    </div>
  </div>
</template>

<style scoped>
.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
  gap: 16px;
  margin-bottom: 32px;
}
.stat-card {
  padding: 18px 20px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  position: relative;
  overflow: hidden;
}
.stat-card::before {
  content: "";
  position: absolute;
  top: 0; left: 0;
  width: 4px; height: 100%;
  background: var(--accent);
}
.stat-card.stat-critical::before { background: var(--danger); }
.stat-card.stat-high::before { background: var(--warning); }
.stat-card.stat-medium::before { background: var(--info); }
.stat-card.stat-low::before { background: var(--text-secondary); }
.stat-label {
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
}
.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-primary);
  font-variant-numeric: tabular-nums;
}
@media (max-width: 640px) {
  .stats-grid {
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 8px;
  }
}
</style>
