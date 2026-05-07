<script setup lang="ts">
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import WheelKpiRow from "../organisms/WheelKpiRow.vue";
import WheelDistributionTable from "../organisms/WheelDistributionTable.vue";
import WheelTopWinnersTable from "../organisms/WheelTopWinnersTable.vue";
import WheelRecentSpinsTable from "../organisms/WheelRecentSpinsTable.vue";
</script>

<template>
  <AdminPageShell title="Roue du Destin — analytics" icon="🪙">
    <template #lede>
      50 derniers spins (toutes guilds) + leaderboard 7 jours +
      distribution des cases tombées sur les spins récents.
    </template>

    <WheelKpiRow />

    <div class="grid">
      <WheelDistributionTable />
      <WheelTopWinnersTable />
    </div>

    <WheelRecentSpinsTable />
  </AdminPageShell>
</template>

<style scoped>
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
