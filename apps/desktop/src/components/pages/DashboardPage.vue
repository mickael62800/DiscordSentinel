<script setup lang="ts">
import { useDashboard } from "../../composables/useDashboard";
import StatCard from "../molecules/StatCard.vue";

const { stats, loading } = useDashboard();
</script>

<template>
  <div class="dashboard">
    <h1>Dashboard</h1>

    <div v-if="!loading && stats" class="stats-grid">
      <StatCard label="Servers" :value="stats.total_servers" color="var(--accent)" />
      <StatCard label="Users" :value="stats.total_users.toLocaleString()" color="var(--info)" />
      <StatCard label="Messages today" :value="stats.messages_today.toLocaleString()" />
      <StatCard label="Infractions today" :value="stats.infractions_today" color="var(--danger)" />
      <StatCard label="Bots online" :value="`${stats.bots_online} / ${stats.bots_total}`" color="var(--success)" />
    </div>

    <div v-else class="loading">Loading...</div>
  </div>
</template>

<style scoped>
.dashboard h1 {
  margin-bottom: 24px;
}

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 16px;
}
</style>
