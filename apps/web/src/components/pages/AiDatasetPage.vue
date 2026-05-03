<script setup lang="ts">
import DatasetFilters from "../organisms/DatasetFilters.vue";
import DatasetStatsBar from "../organisms/DatasetStatsBar.vue";
import DatasetMessagesTable from "../organisms/DatasetMessagesTable.vue";
</script>

<template>
  <div class="dataset-page page--wide">
    <div class="mobile-only-block">
      <div class="mobile-block-card">
        <div class="mobile-block-icon">🖥️</div>
        <h2>Disponible sur desktop uniquement</h2>
        <p>
          La collecte et l'export du dataset IA nécessitent un grand écran
          (tableau dense, sélection multi-lignes, export CSV).
        </p>
        <p class="muted">Ouvre cette page depuis ton ordinateur pour continuer.</p>
      </div>
    </div>

    <div class="page-header">
      <h1>📚 Dataset IA — collecte de messages</h1>
      <p class="muted">
        Sélectionne les messages stockés et étiquette-les manuellement. À l'export, deux fichiers CSV
        (<code>safe</code> et <code>severe</code>) sont téléchargés et les messages exportés sont
        supprimés de la base.
      </p>
    </div>

    <DatasetFilters />
    <DatasetStatsBar />
    <DatasetMessagesTable />
  </div>
</template>

<style scoped>
.dataset-page { padding: 16px; }
.page-header h1 { margin: 0 0 4px; }
.muted { color: var(--text-secondary); font-size: 12px; }
.mobile-only-block { display: none; }
@media (max-width: 768px) {
  .mobile-only-block {
    display: flex; align-items: center; justify-content: center;
    min-height: 60vh;
  }
  .dataset-page > :not(.mobile-only-block) { display: none !important; }
  .mobile-block-card {
    background: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 12px;
    padding: 32px 20px;
    text-align: center;
    max-width: 360px;
  }
  .mobile-block-icon { font-size: 48px; margin-bottom: 12px; }
  .mobile-block-card h2 { margin: 0 0 12px; font-size: 18px; color: var(--text-primary); }
  .mobile-block-card p {
    margin: 0 0 8px;
    font-size: 13px;
    color: var(--text-secondary);
    line-height: 1.5;
  }
}
</style>
