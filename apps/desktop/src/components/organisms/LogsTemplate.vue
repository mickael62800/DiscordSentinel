<script setup lang="ts">
import { computed } from "vue";
import { useLogs } from "../../composables/useLogs";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { levelVariant } from "../../utils/variants";
import type { TableColumn } from "../../types";
import FilterBar from "../molecules/FilterBar.vue";
import DataTable from "../organisms/DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";
import LoadingState from "../atoms/LoadingState.vue";

const props = withDefaults(defineProps<{
  title: string;
  category: string;
  columns: TableColumn[];
  emptyMessage: string;
  showSourceFilter?: boolean;
  sourceLabel?: string;
  showClearButton?: boolean;
  clearConfirmMessage?: string;
}>(), {
  showSourceFilter: false,
  sourceLabel: "Toutes les sources",
  showClearButton: false,
  clearConfirmMessage: "Supprimer tous les journaux ?",
});

const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();
const { filteredLogs, sources, loading, filterLevel, filterBot, dateFrom, dateTo, search, clearLogs } = useLogs(props.category);

const hasDetailsColumn = computed(() => props.columns.some((c) => c.key === "details"));

const filters = computed(() => {
  const list = [
    {
      modelValue: filterLevel.value,
      options: [
        { value: "all", label: "Tous les niveaux" },
        { value: "info", label: "Info" },
        { value: "warn", label: "Avertissement" },
        { value: "error", label: "Erreur" },
      ],
    },
  ];
  if (props.showSourceFilter) {
    list.push({
      modelValue: filterBot.value,
      options: [
        { value: "all", label: props.sourceLabel },
        ...sources.value.map((s) => ({ value: s, label: s })),
      ],
    });
  }
  return list;
});

function onFilterUpdate(index: number, value: string) {
  if (index === 0) filterLevel.value = value;
  if (index === 1) filterBot.value = value;
}

async function handleClear() {
  const ok = await confirm({ message: props.clearConfirmMessage });
  if (!ok) return;
  await clearLogs();
}
</script>

<template>
  <div class="logs">
    <h1>{{ title }}</h1>

    <input v-model="search" type="text" placeholder="Rechercher dans tous les champs..." class="search-global" />

    <div class="filters-row">
      <FilterBar :filters="filters" @update:filter="onFilterUpdate" />
      <div class="date-filters">
        <label>Du <input type="date" v-model="dateFrom" class="date-input" /></label>
        <label>Au <input type="date" v-model="dateTo" class="date-input" /></label>
      </div>
      <button v-if="showClearButton" class="clear-btn" @click="handleClear">Tout supprimer</button>
    </div>

    <LoadingState v-if="loading" />

    <DataTable
      v-else
      :columns="columns"
      :rows="(filteredLogs as unknown as Record<string, unknown>[])"
      :empty-message="emptyMessage"
    >
      <template #cell-timestamp="{ value }">
        <span class="mono">{{ fmt(String(value)) }}</span>
      </template>
      <template #cell-level="{ value }">
        <AppBadge :label="String(value)" :variant="levelVariant(String(value))" />
      </template>
      <template v-if="hasDetailsColumn" #cell-details="{ value }">
        <slot name="details" :value="value">
          <span v-if="value && typeof value === 'object' && Object.keys(value as object).length > 0" class="mono details-text">
            {{ Object.entries(value as Record<string, unknown>).map(([k, v]) => `${k}: ${v}`).join(' | ') }}
          </span>
        </slot>
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
.details-text { font-size: 11px; color: var(--text-secondary); }
</style>
