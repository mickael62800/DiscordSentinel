<script setup lang="ts">
import { computed } from "vue";
import { Doughnut } from "vue-chartjs";
import { useWheelAnalytics } from "@/composables/useWheelAnalytics";
import { registerChartJs } from "@/utils/chartjs";
import { makeDoughnutOptions, colorAt } from "@/utils/chartTheme";

registerChartJs();

const { distribution, totalSpins, loading } = useWheelAnalytics();

// B4 : doughnut compact de la distribution (garde le tableau detaille dessous).
const distributionChartData = computed(() => ({
  labels: distribution.value.map((d) => d.label),
  datasets: [
    {
      data: distribution.value.map((d) => d.count),
      backgroundColor: distribution.value.map((_, i) => colorAt(i)),
      borderWidth: 0,
    },
  ],
}));

const distributionChartOptions = makeDoughnutOptions();
</script>

<template>
  <section class="card">
    <h2>🎲 Distribution des cases</h2>
    <div v-if="loading" class="loading">Chargement…</div>
    <div v-else-if="distribution.length === 0" class="empty">Aucun spin récent.</div>
    <template v-else>
      <div class="dist-chart">
        <Doughnut :data="distributionChartData" :options="distributionChartOptions" />
      </div>
      <table class="table">
      <thead>
        <tr>
          <th>Case</th>
          <th>Tombée</th>
          <th>%</th>
          <th>Payout total</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="d in distribution" :key="d.case_key">
          <td>
            <strong>{{ d.label }}</strong>
            <small class="muted">{{ d.case_key }}</small>
          </td>
          <td>{{ d.count }}</td>
          <td>{{ ((d.count / totalSpins) * 100).toFixed(1) }}%</td>
          <td>{{ d.total_payout.toLocaleString() }}c</td>
        </tr>
      </tbody>
      </table>
    </template>
  </section>
</template>

<style scoped>
@import "../pages/_admin-page-shared.css";

.dist-chart {
  height: 260px;
  position: relative;
  margin-bottom: 20px;
}
</style>
