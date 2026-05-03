<script setup lang="ts">
import { useAutomod } from "@/composables/useAutomod";

const { statsByCategory, topUsers, totalDetections } = useAutomod();
</script>

<template>
  <div>
    <section class="kpi-row">
      <div class="kpi-card">
        <span class="kpi-value">{{ totalDetections }}</span>
        <span class="kpi-label">Détections récentes</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ statsByCategory.length }}</span>
        <span class="kpi-label">Catégories distinctes</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ topUsers.length }}</span>
        <span class="kpi-label">Utilisateurs détectés (top 10)</span>
      </div>
    </section>

    <div class="grid">
      <!-- Stats par catégorie -->
      <section class="card">
        <h2>Catégories</h2>
        <div v-if="statsByCategory.length === 0" class="empty">Aucune détection.</div>
        <ul v-else class="cat-list">
          <li v-for="cat in statsByCategory" :key="cat.key">
            <span class="cat-name">{{ cat.key }}</span>
            <span class="cat-count">{{ cat.count }}</span>
          </li>
        </ul>
      </section>

      <!-- Top users -->
      <section class="card">
        <h2>Top utilisateurs</h2>
        <div v-if="topUsers.length === 0" class="empty">Aucune détection.</div>
        <ul v-else class="user-list">
          <li v-for="user in topUsers" :key="user.user_id">
            <span class="user-name">{{ user.username }}</span>
            <span class="user-id">{{ user.user_id }}</span>
            <span class="user-count">{{ user.count }}</span>
          </li>
        </ul>
      </section>
    </div>
  </div>
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
  display: flex;
  flex-direction: column;
}
.kpi-value { font-size: 1.8rem; font-weight: 700; }
.kpi-label { font-size: 0.85rem; color: var(--text-secondary); margin-top: 4px; }

.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
  margin-bottom: 20px;
}
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 20px;
}
.card h2 { margin: 0 0 12px 0; font-size: 1.1rem; }
.empty { color: var(--text-secondary); font-style: italic; }

.cat-list, .user-list { list-style: none; padding: 0; margin: 0; }
.cat-list li {
  display: flex;
  justify-content: space-between;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
.cat-list li:last-child, .user-list li:last-child { border-bottom: none; }
.cat-count, .user-count { font-weight: 600; color: var(--accent); }

.user-list li {
  display: grid;
  grid-template-columns: 2fr 2fr 1fr;
  gap: 8px;
  align-items: center;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
}
.user-id {
  font-family: monospace;
  font-size: 0.85rem;
  color: var(--text-secondary);
}
.user-count { text-align: right; }

@media (max-width: 640px) {
  .kpi-row { grid-template-columns: 1fr; gap: 8px; }
  .kpi-card { padding: 12px 14px; }
  .kpi-value { font-size: 1.4rem; }
  .grid { grid-template-columns: 1fr; gap: 12px; }
  .card { padding: 14px; }
  .user-list li {
    grid-template-columns: 1fr auto;
    row-gap: 2px;
  }
  .user-id { grid-column: 1 / -1; font-size: 0.75rem; }
}
</style>
