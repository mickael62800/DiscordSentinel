<script setup lang="ts">
import { useDashboard } from "@/composables/useDashboard";
import StatCard from "../molecules/StatCard.vue";

const { stats, loading, error, fetchStats } = useDashboard();

defineExpose({ refresh: fetchStats });
</script>

<template>
  <section class="dash-section">
    <h2 class="section-title">Etat du systeme</h2>

    <div v-if="!loading && stats" class="stats-grid">
      <StatCard label="Serveurs" :value="stats.total_servers" color="var(--accent)" />
      <StatCard label="Utilisateurs" :value="stats.total_users.toLocaleString()" color="var(--info)" />
      <StatCard label="Messages aujourd'hui" :value="stats.messages_today.toLocaleString()" />
      <StatCard label="Infractions aujourd'hui" :value="stats.infractions_today" color="var(--danger)" />
      <StatCard label="Bots en ligne" :value="`${stats.bots_online} / ${stats.bots_total}`" color="var(--success)" />
      <StatCard label="Workers en ligne" :value="`${stats.workers_online} / ${stats.workers_total}`" color="var(--accent)" />
      <StatCard label="PostgreSQL" :value="stats.postgres_online ? 'En ligne' : 'Hors ligne'" :color="stats.postgres_online ? 'var(--success)' : 'var(--danger)'" />
      <StatCard label="Redis" :value="stats.redis_online ? 'En ligne' : 'Hors ligne'" :color="stats.redis_online ? 'var(--success)' : 'var(--danger)'" />
    </div>
    <div v-else-if="loading" class="loading">Chargement des stats...</div>
    <div v-else-if="error" class="error-msg">Erreur chargement stats : {{ error }}</div>
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

.stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.error-msg {
  color: var(--danger);
  background-color: var(--danger-bg);
  border: 1px solid var(--danger);
  border-radius: 8px;
  padding: 12px 16px;
  font-size: 13px;
}

.loading {
  color: var(--text-secondary);
  padding: 30px;
  text-align: center;
}
</style>
