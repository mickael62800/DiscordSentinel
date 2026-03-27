<script setup lang="ts">
import { computed } from "vue";
import { useLogs } from "../../composables/useLogs";
import type { TableColumn } from "../../types";
import FilterBar from "../molecules/FilterBar.vue";
import DataTable from "../organisms/DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";

const { filteredLogs, bots, loading, filterLevel, filterBot } = useLogs();

const columns: TableColumn[] = [
  { key: "timestamp", label: "Heure" },
  { key: "level", label: "Niveau" },
  { key: "bot", label: "Bot" },
  { key: "server", label: "Serveur" },
  { key: "message", label: "Message" },
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
      { value: "all", label: "Tous les bots" },
      ...bots.value.map((b) => ({ value: b, label: b })),
    ],
  },
]);

function onFilterUpdate(index: number, value: string) {
  if (index === 0) filterLevel.value = value;
  if (index === 1) filterBot.value = value;
}

function levelVariant(level: string): "info" | "warn" | "error" | "default" {
  if (level === "info" || level === "warn" || level === "error") return level;
  return "default";
}
</script>

<template>
  <div class="logs">
    <h1>Journaux</h1>

    <FilterBar :filters="filters" @update:filter="onFilterUpdate" />

    <div v-if="loading" class="loading">Chargement...</div>

    <DataTable
      v-else
      :columns="columns"
      :rows="(filteredLogs as unknown as Record<string, unknown>[])"
      empty-message="Aucun journal correspondant aux filtres"
    >
      <template #cell-timestamp="{ value }">
        <span class="mono">{{ value }}</span>
      </template>
      <template #cell-level="{ value }">
        <AppBadge :label="String(value)" :variant="levelVariant(String(value))" />
      </template>
    </DataTable>
  </div>
</template>

<style scoped>
.logs h1 {
  margin-bottom: 24px;
}

</style>
