<script setup lang="ts">
import { computed } from "vue";
import { useLogs } from "../../composables/useLogs";
import type { TableColumn } from "../../types";
import FilterBar from "../molecules/FilterBar.vue";
import DataTable from "../organisms/DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";

const { filteredLogs, bots, loading, filterLevel, filterBot } = useLogs();

const columns: TableColumn[] = [
  { key: "timestamp", label: "Time" },
  { key: "level", label: "Level" },
  { key: "bot", label: "Bot" },
  { key: "server", label: "Server" },
  { key: "message", label: "Message" },
];

const filters = computed(() => [
  {
    modelValue: filterLevel.value,
    options: [
      { value: "all", label: "All levels" },
      { value: "info", label: "Info" },
      { value: "warn", label: "Warning" },
      { value: "error", label: "Error" },
    ],
  },
  {
    modelValue: filterBot.value,
    options: [
      { value: "all", label: "All bots" },
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
    <h1>Logs</h1>

    <FilterBar :filters="filters" @update:filter="onFilterUpdate" />

    <div v-if="loading" class="loading">Loading...</div>

    <DataTable
      v-else
      :columns="columns"
      :rows="(filteredLogs as unknown as Record<string, unknown>[])"
      empty-message="No logs matching filters"
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
