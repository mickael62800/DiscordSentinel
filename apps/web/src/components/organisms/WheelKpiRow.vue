<script setup lang="ts">
import { useWheelAnalytics } from "@/composables/useWheelAnalytics";

const { totalSpins, totalPayout, avgPayout } = useWheelAnalytics();
</script>

<template>
  <section class="kpi-row">
    <div class="kpi-card">
      <span class="kpi-value">{{ totalSpins }}</span>
      <span class="kpi-label">Spins récents</span>
    </div>
    <div class="kpi-card">
      <span class="kpi-value">{{ totalPayout.toLocaleString() }}c</span>
      <span class="kpi-label">Total payout (récent)</span>
    </div>
    <div class="kpi-card">
      <span class="kpi-value">{{ avgPayout.toLocaleString() }}c</span>
      <span class="kpi-label">Moyenne / spin</span>
    </div>
  </section>
</template>

<style scoped>
.kpi-row {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin-bottom: 20px;
}
.kpi-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px 20px;
}
.kpi-value { font-size: 1.6rem; font-weight: 700; display: block; }
.kpi-label { font-size: 0.85rem; color: var(--text-secondary); }
@media (max-width: 640px) {
  .kpi-row { grid-template-columns: 1fr; gap: 8px; }
}
</style>
