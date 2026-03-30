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
    title="Journaux API"
    category="api"
    :columns="columns"
    empty-message="Aucun journal API"
    show-clear-button
    clear-confirm-message="Supprimer tous les journaux API ?"
  >
    <template #details="{ value }">
      <div v-if="value && typeof value === 'object'" class="details-cell">
        <span v-if="(value as Record<string, unknown>).method" class="detail-tag method">{{ (value as Record<string, unknown>).method }}</span>
        <span v-if="(value as Record<string, unknown>).route" class="detail-tag route">{{ (value as Record<string, unknown>).route }}</span>
        <span v-if="(value as Record<string, unknown>).status_code" class="detail-tag" :class="Number((value as Record<string, unknown>).status_code) >= 500 ? 'status-error' : Number((value as Record<string, unknown>).status_code) >= 400 ? 'status-warn' : 'status-ok'">
          {{ (value as Record<string, unknown>).status_code }} {{ (value as Record<string, unknown>).status_text }}
        </span>
        <span v-if="(value as Record<string, unknown>).latency_ms" class="detail-tag latency">{{ (value as Record<string, unknown>).latency_ms }}ms</span>
        <span v-if="(value as Record<string, unknown>).event" class="detail-tag event">{{ (value as Record<string, unknown>).event }}</span>
      </div>
    </template>
  </LogsTemplate>
</template>

<style scoped>
.details-cell { display: flex; gap: 6px; flex-wrap: wrap; align-items: center; }
.detail-tag { padding: 2px 8px; border-radius: 4px; font-size: 11px; font-family: monospace; font-weight: 600; }
.method { background: var(--accent-bg); color: #5865f2; }
.route { background: var(--muted-bg); color: var(--text-secondary); }
.status-ok { background: var(--success-bg); color: #57f287; }
.status-warn { background: var(--warning-bg); color: #f59e0b; }
.status-error { background: var(--danger-bg); color: #ed4245; }
.latency { background: var(--muted-bg); color: var(--text-secondary); }
.event { background: var(--accent-bg); color: var(--accent); }
</style>
