<script setup lang="ts">
import { useConfessions } from "@/composables/useConfessions";
import ConfessionsTable from "../organisms/ConfessionsTable.vue";
import ConfessionsReportsTable from "../organisms/ConfessionsReportsTable.vue";
import ConfessionRepliesModal from "../organisms/ConfessionRepliesModal.vue";

const { tab, showDeleted, confessions, reports, loading } = useConfessions();
</script>

<template>
  <div class="confessions-page page--wide">
    <header class="page-head">
      <div>
        <h1>📝 Modération des confessions</h1>
        <p class="muted small">
          Confessions anonymes postées via /confess. Seul le owner voit l'auteur réel.
        </p>
      </div>
      <div class="actions">
        <label class="cb">
          <input v-model="showDeleted" type="checkbox" />
          <span>Afficher supprimées</span>
        </label>
      </div>
    </header>

    <div class="tabs">
      <button :class="['tab', { active: tab === 'confessions' }]" @click="tab = 'confessions'">
        Confessions ({{ confessions.length }})
      </button>
      <button :class="['tab', { active: tab === 'reports' }]" @click="tab = 'reports'">
        🚩 Signalements ({{ reports.length }})
      </button>
    </div>

    <div v-if="loading" class="muted">Chargement…</div>
    <ConfessionsTable v-else-if="tab === 'confessions'" />
    <ConfessionsReportsTable v-else-if="tab === 'reports'" />

    <ConfessionRepliesModal />
  </div>
</template>

<style scoped>
.confessions-page { padding: 0; }
.page-head { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 16px; }
.page-head h1 { margin: 0; font-size: 24px; }
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
.tabs { display: flex; gap: 4px; margin-bottom: 16px; border-bottom: 1px solid var(--border); }
.tab { background: transparent; border: 0; padding: 10px 16px; font-size: 13px; color: var(--text-secondary); cursor: pointer; border-bottom: 2px solid transparent; font-weight: 600; }
.tab:hover { color: var(--text-primary); }
.tab.active { color: var(--accent); border-bottom-color: var(--accent); }
.cb { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; }
</style>
