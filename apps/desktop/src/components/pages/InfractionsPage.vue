<script setup lang="ts">
import { useInfractions } from "../../composables/useInfractions";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { useSearch } from "../../composables/useSearch";
import type { TableColumn, Infraction } from "../../types";
import DataTable from "../organisms/DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";
import AppInput from "../atoms/AppInput.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import { infractionTypeVariant } from "../../utils/variants";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { infractions, loading, error, fetchInfractions } = useInfractions();
useRealtimeRefresh(["infraction_new"], fetchInfractions);
const { search, filtered: filteredInfractions } = useSearch<Infraction>(
  infractions,
  ["username", "user_id", "reason", "infraction_type", "moderator", "server", "created_at"],
);

const columns: TableColumn[] = [
  { key: "username", label: "Utilisateur" },
  { key: "infraction_type", label: "Type" },
  { key: "reason", label: "Raison" },
  { key: "moderator", label: "Moderateur" },
  { key: "created_at", label: "Date" },
];
</script>

<template>
  <div class="infractions">
    <h1>Infractions</h1>

    <div class="toolbar">
      <AppInput
        v-model="search"
        placeholder="Rechercher dans tous les champs..."
      />
    </div>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchInfractions" />
    <LoadingState v-else-if="loading" />

    <DataTable
      v-else
      :columns="columns"
      :rows="(filteredInfractions as unknown as Record<string, unknown>[])"
      empty-message="Aucune infraction"
    >
      <template #cell-username="{ row }">
        <div class="user-cell">
          <span class="username">{{ (row as Record<string, unknown>).username }}</span>
          <span class="user-id">{{ (row as Record<string, unknown>).user_id }}</span>
        </div>
      </template>
      <template #cell-infraction_type="{ value }">
        <AppBadge :label="String(value)" :variant="infractionTypeVariant(String(value))" />
      </template>
      <template #cell-created_at="{ value }">
        <span class="mono">{{ fmt(String(value)) }}</span>
      </template>
    </DataTable>
  </div>
</template>

<style scoped>
.infractions h1 {
  margin-bottom: 24px;
}

.toolbar {
  display: flex;
  margin-bottom: 16px;
}

.toolbar input {
  max-width: 360px;
}

.user-cell {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.username {
  font-weight: 600;
}

.user-id {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: monospace;
}
</style>
