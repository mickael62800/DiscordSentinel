<script setup lang="ts">
import { ref } from "vue";
import type { TableColumn } from "../../types";
import LogsTemplate from "../organisms/LogsTemplate.vue";

interface Tab {
  key: string;
  label: string;
  category: string;
  columns: TableColumn[];
  showSourceFilter?: boolean;
  sourceLabel?: string;
  showClearButton?: boolean;
  clearConfirmMessage?: string;
}

const activeTab = ref("discord");

const tabs: Tab[] = [
  {
    key: "discord",
    label: "Discord",
    category: "discord",
    columns: [
      { key: "timestamp", label: "Date" },
      { key: "level", label: "Niveau" },
      { key: "bot", label: "Source" },
      { key: "server", label: "Serveur" },
      { key: "message", label: "Message" },
    ],
    // Plus qu'une seule source en pratique -> filtre inutile, on le cache.
    showSourceFilter: false,
  },
  {
    key: "bots",
    label: "Bots",
    category: "bot",
    columns: [
      { key: "timestamp", label: "Date" },
      { key: "level", label: "Niveau" },
      { key: "bot", label: "Bot" },
      { key: "message", label: "Message" },
      { key: "details", label: "Détails" },
    ],
    showSourceFilter: true,
    sourceLabel: "Tous les bots",
    showClearButton: true,
    clearConfirmMessage: "Supprimer tous les journaux bots ?",
  },
  {
    key: "workers",
    label: "Workers",
    category: "worker",
    columns: [
      { key: "timestamp", label: "Date" },
      { key: "level", label: "Niveau" },
      { key: "bot", label: "Worker" },
      { key: "message", label: "Message" },
      { key: "details", label: "Détails" },
    ],
    showSourceFilter: true,
    sourceLabel: "Tous les workers",
    showClearButton: true,
    clearConfirmMessage: "Supprimer tous les journaux workers ?",
  },
  {
    key: "api",
    label: "API",
    category: "api",
    columns: [
      { key: "timestamp", label: "Date" },
      { key: "level", label: "Niveau" },
      { key: "message", label: "Message" },
      { key: "details", label: "Détails" },
    ],
    showClearButton: true,
    clearConfirmMessage: "Supprimer tous les journaux API ?",
  },
  {
    key: "websocket",
    label: "WebSocket",
    category: "websocket",
    columns: [
      { key: "timestamp", label: "Date" },
      { key: "level", label: "Niveau" },
      { key: "message", label: "Message" },
      { key: "details", label: "Détails" },
    ],
    showClearButton: true,
    clearConfirmMessage: "Supprimer tous les journaux WebSocket ?",
  },
];

function statusClass(code: unknown): string {
  const n = Number(code);
  if (n >= 500) return "status-error";
  if (n >= 400) return "status-warn";
  return "status-ok";
}
</script>

<template>
  <div class="logs-page">
    <h1 class="page-title">Journaux</h1>

    <div class="tab-bar">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="tab-btn"
        :class="{ active: activeTab === tab.key }"
        @click="activeTab = tab.key"
      >
        {{ tab.label }}
      </button>
    </div>

    <div class="tab-content">
      <template v-for="tab in tabs" :key="tab.key">
        <LogsTemplate
          v-if="activeTab === tab.key"
          :title="tab.label"
          :category="tab.category"
          :columns="tab.columns"
          :empty-message="`Aucun journal ${tab.label.toLowerCase()} trouvé.`"
          :show-source-filter="tab.showSourceFilter ?? false"
          :source-label="tab.sourceLabel ?? 'Toutes les sources'"
          :show-clear-button="tab.showClearButton ?? false"
          :clear-confirm-message="tab.clearConfirmMessage ?? 'Supprimer tous les journaux ?'"
        >
          <!-- API custom details slot -->
          <template v-if="tab.key === 'api'" #details="{ value }">
            <div v-if="value && typeof value === 'object'" class="details-cell">
              <span v-if="(value as Record<string, unknown>).method" class="detail-tag method">
                {{ (value as Record<string, unknown>).method }}
              </span>
              <span v-if="(value as Record<string, unknown>).route" class="detail-tag route">
                {{ (value as Record<string, unknown>).route }}
              </span>
              <span
                v-if="(value as Record<string, unknown>).status_code"
                class="detail-tag"
                :class="statusClass((value as Record<string, unknown>).status_code)"
              >
                {{ (value as Record<string, unknown>).status_code }}
                <template v-if="(value as Record<string, unknown>).status_text">
                  {{ (value as Record<string, unknown>).status_text }}
                </template>
              </span>
              <span v-if="(value as Record<string, unknown>).latency_ms" class="detail-tag latency">
                {{ (value as Record<string, unknown>).latency_ms }}ms
              </span>
              <span v-if="(value as Record<string, unknown>).event" class="detail-tag event">
                {{ (value as Record<string, unknown>).event }}
              </span>
            </div>
          </template>

          <!-- WebSocket custom details slot -->
          <template v-if="tab.key === 'websocket'" #details="{ value }">
            <div v-if="value && typeof value === 'object'" class="details-cell">
              <span v-if="(value as Record<string, unknown>).event" class="detail-tag event">
                {{ (value as Record<string, unknown>).event }}
              </span>
              <span v-if="(value as Record<string, unknown>).client_ip" class="detail-tag ip">
                {{ (value as Record<string, unknown>).client_ip }}
              </span>
              <span v-if="(value as Record<string, unknown>).total_clients != null" class="detail-tag clients">
                {{ (value as Record<string, unknown>).total_clients }} clients
              </span>
              <span v-if="(value as Record<string, unknown>).events_relayed != null" class="detail-tag relayed">
                {{ (value as Record<string, unknown>).events_relayed }} relayed
              </span>
              <span v-if="(value as Record<string, unknown>).skipped_events != null" class="detail-tag skipped">
                {{ (value as Record<string, unknown>).skipped_events }} skipped
              </span>
            </div>
          </template>
        </LogsTemplate>
      </template>
    </div>
  </div>
</template>

<style scoped>
.logs-page {
  padding: 0;
}

.page-title {
  margin-bottom: 20px;
}

.tab-bar {
  display: flex;
  gap: 0;
  border-bottom: 2px solid var(--border);
  margin-bottom: 24px;
}

.tab-btn {
  padding: 10px 20px;
  background: transparent;
  border: none;
  border-bottom: 2px solid transparent;
  margin-bottom: -2px;
  color: var(--text-secondary);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
  transition: color var(--transition-base), border-color var(--transition-base);
}

.tab-btn:hover {
  color: var(--text-primary);
}

.tab-btn.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.tab-content {
  min-height: 0;
}

/* Detail tag styles for API and WebSocket tabs */
.details-cell {
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
  align-items: center;
}

.detail-tag {
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  font-family: monospace;
  font-weight: 600;
}

/* API */
.method {
  background: var(--accent-bg);
  color: var(--accent);
}

.route {
  background: var(--muted-bg);
  color: var(--text-secondary);
}

.status-ok {
  background: var(--success-bg);
  color: var(--success);
}

.status-warn {
  background: var(--warning-bg);
  color: var(--warning);
}

.status-error {
  background: var(--danger-bg);
  color: var(--danger);
}

.latency {
  background: var(--muted-bg);
  color: var(--text-secondary);
}

/* WebSocket */
.event {
  background: var(--accent-bg);
  color: var(--accent);
}

.ip {
  background: var(--muted-bg);
  color: var(--text-secondary);
}

.clients {
  background: var(--success-bg);
  color: #57f287;
}

.relayed {
  background: var(--warning-bg);
  color: #f59e0b;
}

.skipped {
  background: var(--danger-bg);
  color: #ed4245;
}
</style>
