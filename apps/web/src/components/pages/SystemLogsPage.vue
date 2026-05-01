<script setup lang="ts">
import LogsColumn from "../organisms/LogsColumn.vue";
</script>

<template>
  <div class="system-logs">
    <h1 class="page-title">⚙️ Logs système</h1>
    <p class="page-hint">
      Logs des bots Discord, workers, requêtes API et WebSocket — affichés
      en parallèle pour faciliter le diagnostic d'incidents qui touchent
      plusieurs catégories en même temps.
    </p>

    <div class="system-grid">
      <LogsColumn title="Bots" category="bot" />
      <LogsColumn title="Workers" category="worker" />
      <LogsColumn title="API" category="api" />
      <LogsColumn title="WebSocket" category="websocket" />
    </div>
  </div>
</template>

<style scoped>
.system-logs { padding: 0; }
.page-title { margin: 0 0 8px 0; font-size: 24px; }
.page-hint {
  color: var(--text-secondary);
  font-size: 13px;
  margin: 0 0 20px 0;
}

.system-grid {
  display: grid;
  grid-template-columns: repeat(4, minmax(0, 1fr));
  gap: 16px;
  align-items: start;
}
@media (max-width: 1400px) {
  .system-grid { grid-template-columns: repeat(2, minmax(0, 1fr)); }
}
@media (max-width: 800px) {
  .system-grid { grid-template-columns: 1fr; }
}
</style>
