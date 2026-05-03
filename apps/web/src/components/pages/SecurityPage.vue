<script setup lang="ts">
import SecurityStatsGrid from "../organisms/SecurityStatsGrid.vue";
import SecurityEventsList from "../organisms/SecurityEventsList.vue";
</script>

<template>
  <div class="security page--wide">
    <header class="page-header">
      <h1>Evenements de securite</h1>
      <p class="page-subtitle">Detection des raids, comptes suspects et actions massives</p>
    </header>

    <SecurityStatsGrid />
    <SecurityEventsList />
  </div>
</template>

<style scoped>
.security { padding: 24px; }
.page-header { margin-bottom: 24px; }
.page-header h1 {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-primary);
}
.page-subtitle {
  color: var(--text-secondary);
  font-size: 14px;
  margin-top: 4px;
}
</style>
