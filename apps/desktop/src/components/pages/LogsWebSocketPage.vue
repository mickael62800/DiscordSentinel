<script setup lang="ts">
import { computed } from "vue";
import { useLogs } from "../../composables/useLogs";
import type { TableColumn } from "../../types";
import FilterBar from "../molecules/FilterBar.vue";
import DataTable from "../organisms/DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";
import { levelVariant } from "../../utils/variants";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { filteredLogs, loading, filterLevel, dateFrom, dateTo, search, clearLogs } = useLogs("websocket");

async function handleClear() {
  if (!confirm("Supprimer tous les journaux WebSocket ?")) return;
  await clearLogs();
}

const columns: TableColumn[] = [
  { key: "timestamp", label: "Heure" },
  { key: "level", label: "Niveau" },
  { key: "message", label: "Message" },
  { key: "details", label: "Details" },
];

const filters = computed(() => [
  {
    modelValue: filterLevel.value,
    options: [
      { value: "all", label: "Tous les niveaux" },
      { value: "info", label: "Info" },
      { value: "warn", label: "Avertissement" },
      { value: "error", label: "Erreur" },
    ],
  },
]);

function onFilterUpdate(index: number, value: string) {
  if (index === 0) filterLevel.value = value;
}
</script>

<template>
  <div class="logs">
    <h1>Journaux WebSocket</h1>

    <input v-model="search" type="text" placeholder="Rechercher dans tous les champs..." class="search-global" />

    <div class="filters-row">
      <FilterBar :filters="filters" @update:filter="onFilterUpdate" />
      <div class="date-filters">
        <label>Du <input type="date" v-model="dateFrom" class="date-input" /></label>
        <label>Au <input type="date" v-model="dateTo" class="date-input" /></label>
      </div>
      <button class="clear-btn" @click="handleClear">Tout supprimer</button>
    </div>

    <div v-if="loading" class="loading">Chargement...</div>

    <DataTable
      v-else
      :columns="columns"
      :rows="(filteredLogs as unknown as Record<string, unknown>[])"
      empty-message="Aucun journal WebSocket"
    >
      <template #cell-timestamp="{ value }">
        <span class="mono">{{ fmt(String(value)) }}</span>
      </template>
      <template #cell-level="{ value }">
        <AppBadge :label="String(value)" :variant="levelVariant(String(value))" />
      </template>
      <template #cell-details="{ value }">
        <div v-if="value && typeof value === 'object'" class="details-cell">
          <span v-if="(value as Record<string, unknown>).event" class="detail-tag event">{{ (value as Record<string, unknown>).event }}</span>
          <span v-if="(value as Record<string, unknown>).client_ip" class="detail-tag ip">{{ (value as Record<string, unknown>).client_ip }}</span>
          <span v-if="(value as Record<string, unknown>).total_clients !== undefined" class="detail-tag clients">{{ (value as Record<string, unknown>).total_clients }} clients</span>
          <span v-if="(value as Record<string, unknown>).events_relayed" class="detail-tag relayed">{{ (value as Record<string, unknown>).events_relayed }} events</span>
          <span v-if="(value as Record<string, unknown>).skipped_events" class="detail-tag skipped">{{ (value as Record<string, unknown>).skipped_events }} sautes</span>
        </div>
      </template>
    </DataTable>
  </div>
</template>

<style scoped>
.logs h1 { margin-bottom: 24px; }
.search-global { width: 100%; padding: 10px 14px; margin-bottom: 12px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-card); color: var(--text-primary); font-size: 14px; outline: none; }
.search-global:focus { border-color: var(--accent); }
.search-global::placeholder { color: var(--text-secondary); opacity: 0.6; }
.filters-row { display: flex; align-items: flex-start; gap: 16px; flex-wrap: wrap; margin-bottom: 16px; }
.date-filters { display: flex; gap: 12px; align-items: center; }
.date-filters label { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--text-secondary); }
.date-input { padding: 6px 10px; border: 1px solid var(--border); border-radius: 6px; background: var(--bg-card); color: var(--text-primary); font-size: 13px; font-family: monospace; }
.date-input:focus { outline: none; border-color: var(--accent); }
.clear-btn { margin-left: auto; padding: 8px 16px; background: var(--danger); color: white; border: none; border-radius: 6px; font-size: 13px; font-weight: 600; cursor: pointer; }
.clear-btn:hover { opacity: 0.85; }
.details-cell { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; }
.detail-tag { padding: 2px 8px; border-radius: 4px; font-size: 11px; font-family: monospace; font-weight: 600; }
.event { background: rgba(88, 101, 242, 0.15); color: #5865f2; }
.ip { background: rgba(148, 149, 176, 0.1); color: var(--text-secondary); }
.clients { background: rgba(87, 242, 135, 0.15); color: #57f287; }
.relayed { background: rgba(254, 231, 92, 0.15); color: #f59e0b; }
.skipped { background: rgba(237, 66, 69, 0.15); color: #ed4245; }
</style>
