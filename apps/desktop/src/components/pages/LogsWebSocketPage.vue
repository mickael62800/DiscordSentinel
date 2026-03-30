<script setup lang="ts">
import type { TableColumn } from "../../types";
import LogsTemplate from "../organisms/LogsTemplate.vue";

const columns: TableColumn[] = [
  { key: "timestamp", label: "Heure" },
  { key: "level", label: "Niveau" },
  { key: "message", label: "Message" },
  { key: "details", label: "Details" },
];
</script>

<template>
  <LogsTemplate
    title="Journaux WebSocket"
    category="websocket"
    :columns="columns"
    empty-message="Aucun journal WebSocket"
    show-clear-button
    clear-confirm-message="Supprimer tous les journaux WebSocket ?"
  >
    <template #details="{ value }">
      <div v-if="value && typeof value === 'object'" class="details-cell">
        <span v-if="(value as Record<string, unknown>).event" class="detail-tag event">{{ (value as Record<string, unknown>).event }}</span>
        <span v-if="(value as Record<string, unknown>).client_ip" class="detail-tag ip">{{ (value as Record<string, unknown>).client_ip }}</span>
        <span v-if="(value as Record<string, unknown>).total_clients !== undefined" class="detail-tag clients">{{ (value as Record<string, unknown>).total_clients }} clients</span>
        <span v-if="(value as Record<string, unknown>).events_relayed" class="detail-tag relayed">{{ (value as Record<string, unknown>).events_relayed }} events</span>
        <span v-if="(value as Record<string, unknown>).skipped_events" class="detail-tag skipped">{{ (value as Record<string, unknown>).skipped_events }} sautes</span>
      </div>
    </template>
  </LogsTemplate>
</template>

<style scoped>
.details-cell { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; }
.detail-tag { padding: 2px 8px; border-radius: 4px; font-size: 11px; font-family: monospace; font-weight: 600; }
.event { background: var(--accent-bg); color: #5865f2; }
.ip { background: var(--muted-bg); color: var(--text-secondary); }
.clients { background: var(--success-bg); color: #57f287; }
.relayed { background: var(--warning-bg); color: #f59e0b; }
.skipped { background: var(--danger-bg); color: #ed4245; }
</style>
