<script setup lang="ts">
import WheelKpiRow from "../organisms/WheelKpiRow.vue";
import WheelDistributionTable from "../organisms/WheelDistributionTable.vue";
import WheelTopWinnersTable from "../organisms/WheelTopWinnersTable.vue";
import WheelRecentSpinsTable from "../organisms/WheelRecentSpinsTable.vue";
</script>

<template>
  <div class="page page--constrained">
    <header class="page-header">
      <h1>🪙 Roue du Destin — analytics</h1>
      <p class="lede">
        50 derniers spins (toutes guilds) + leaderboard 7 jours +
        distribution des cases tombées sur les spins récents.
      </p>
    </header>

    <WheelKpiRow />

    <div class="grid">
      <WheelDistributionTable />
      <WheelTopWinnersTable />
    </div>

    <WheelRecentSpinsTable />
  </div>
</template>

<style scoped>
@import "./_admin-page-shared.css";
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 20px;
}
@media (max-width: 640px) {
  .grid { grid-template-columns: 1fr; gap: 12px; }
}
</style>
