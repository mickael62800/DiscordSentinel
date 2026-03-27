<script setup lang="ts">
import { useInfractions } from "../../composables/useInfractions";
import type { TableColumn } from "../../types";
import DataTable from "../organisms/DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";

const { infractions, loading } = useInfractions();

const columns: TableColumn[] = [
  { key: "username", label: "Utilisateur" },
  { key: "server", label: "Serveur" },
  { key: "infraction_type", label: "Type" },
  { key: "reason", label: "Raison" },
  { key: "moderator", label: "Moderateur" },
  { key: "created_at", label: "Date" },
];

function typeVariant(type: string): "danger" | "warning" | "info" | "default" {
  switch (type) {
    case "ban": return "danger";
    case "mute": return "warning";
    case "warn": return "info";
    default: return "default";
  }
}
</script>

<template>
  <div class="infractions">
    <h1>Infractions</h1>

    <div v-if="loading" class="loading">Chargement...</div>

    <DataTable
      v-else
      :columns="columns"
      :rows="(infractions as unknown as Record<string, unknown>[])"
      empty-message="Aucune infraction"
    >
      <template #cell-username="{ row }">
        <div class="user-cell">
          <span class="username">{{ (row as Record<string, unknown>).username }}</span>
          <span class="user-id">{{ (row as Record<string, unknown>).user_id }}</span>
        </div>
      </template>
      <template #cell-infraction_type="{ value }">
        <AppBadge :label="String(value)" :variant="typeVariant(String(value))" />
      </template>
      <template #cell-created_at="{ value }">
        <span class="mono">{{ value }}</span>
      </template>
    </DataTable>
  </div>
</template>

<style scoped>
.infractions h1 {
  margin-bottom: 24px;
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
