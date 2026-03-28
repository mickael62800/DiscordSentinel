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
const { filteredLogs, sources, loading, filterLevel, filterBot, dateFrom, dateTo, search, clearLogs } = useLogs("worker");

async function handleClear() {
  if (!confirm("Supprimer tous les journaux workers ?")) return;
  await clearLogs();
}

const columns: TableColumn[] = [
  { key: "timestamp", label: "Heure" },
  { key: "level", label: "Niveau" },
  { key: "bot", label: "Worker" },
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
  {
    modelValue: filterBot.value,
    options: [
      { value: "all", label: "Tous les workers" },
      ...sources.value.map((w) => ({ value: w, label: w })),
    ],
  },
]);

function onFilterUpdate(index: number, value: string) {
  if (index === 0) filterLevel.value = value;
  if (index === 1) filterBot.value = value;
}
</script>

<template>
  <div class="logs">
    <h1>Journaux Workers</h1>

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
      empty-message="Aucun journal worker"
    >
      <template #cell-timestamp="{ value }">
        <span class="mono">{{ fmt(String(value)) }}</span>
      </template>
      <template #cell-level="{ value }">
        <AppBadge :label="String(value)" :variant="levelVariant(String(value))" />
      </template>
      <template #cell-details="{ value }">
        <span v-if="value && typeof value === 'object' && Object.keys(value as object).length > 0" class="mono details-text">
          {{ Object.entries(value as Record<string, unknown>).map(([k, v]) => `${k}: ${v}`).join(' | ') }}
        </span>
      </template>
    </DataTable>
  </div>
</template>

<style scoped>
.logs h1 { margin-bottom: 24px; }
.search-global { width: 100%; padding: 10px 14px; margin-bottom: 12px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-card); color: var(--text-primary); font-size: 14px; outline: none; }
.search-global:focus { border-color: var(--accent); }
.search-global::placeholder { color: var(--text-secondary); opacity: 0.6; }
.details-text { font-size: 11px; color: var(--text-secondary); }
.clear-btn { margin-left: auto; padding: 8px 16px; background: var(--danger); color: white; border: none; border-radius: 6px; font-size: 13px; font-weight: 600; cursor: pointer; }
.clear-btn:hover { opacity: 0.85; }
.filters-row { display: flex; align-items: flex-start; gap: 16px; flex-wrap: wrap; margin-bottom: 16px; }
.date-filters { display: flex; gap: 12px; align-items: center; }
.date-filters label { display: flex; align-items: center; gap: 6px; font-size: 13px; color: var(--text-secondary); }
.date-input { padding: 6px 10px; border: 1px solid var(--border); border-radius: 6px; background: var(--bg-card); color: var(--text-primary); font-size: 13px; font-family: monospace; }
.date-input:focus { outline: none; border-color: var(--accent); }
</style>
