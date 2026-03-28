<script setup lang="ts">
import { ref, computed } from "vue";
import { useInfractions } from "../../composables/useInfractions";
import type { TableColumn } from "../../types";
import DataTable from "../organisms/DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";
import AppInput from "../atoms/AppInput.vue";
import { infractionTypeVariant } from "../../utils/variants";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { infractions, loading } = useInfractions();
const search = ref("");

const filteredInfractions = computed(() => {
  const term = String(search.value).trim().toLowerCase();
  if (!term) return infractions.value;
  return infractions.value.filter((i) =>
    [i.username, i.user_id, i.reason, i.infraction_type, i.moderator, i.server, i.created_at]
      .some((field) => field?.toLowerCase().includes(term)),
  );
});

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

    <div v-if="loading" class="loading">Chargement...</div>

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
