<script setup lang="ts">
import { useAuditLogs } from "../../composables/useAuditLogs";
import AppBadge from "../atoms/AppBadge.vue";
import type { BadgeVariant } from "../../utils/variants";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { filteredLogs, eventTypes, loading, filterEventType, searchQuery } = useAuditLogs();

function eventVariant(type: string): BadgeVariant {
  switch (type) {
    case "member_ban":
    case "channel_delete":
      return "danger";
    case "member_leave":
    case "message_delete":
    case "member_roles_update":
      return "warning";
    case "member_join":
    case "member_unban":
    case "channel_create":
      return "success";
    case "voice_join":
    case "voice_leave":
    case "voice_move":
      return "info";
    default:
      return "default";
  }
}

function eventLabel(type: string): string {
  const labels: Record<string, string> = {
    message_delete: "Message supprime",
    message_edit: "Message edite",
    member_join: "Membre rejoint",
    member_leave: "Membre parti",
    member_ban: "Membre banni",
    member_unban: "Membre debanni",
    member_roles_update: "Roles modifies",
    voice_join: "Rejoint vocal",
    voice_leave: "Quitte vocal",
    voice_move: "Change de vocal",
    channel_create: "Salon cree",
    channel_delete: "Salon supprime",
  };
  return labels[type] ?? type;
}

function eventIcon(type: string): string {
  const icons: Record<string, string> = {
    message_delete: "X",
    message_edit: "E",
    member_join: "+",
    member_leave: "-",
    member_ban: "B",
    member_unban: "U",
    member_roles_update: "R",
    voice_join: "V",
    voice_leave: "V",
    voice_move: "M",
    channel_create: "#",
    channel_delete: "#",
  };
  return icons[type] ?? "?";
}
</script>

<template>
  <div class="audit">
    <h1>Journal d'audit</h1>

    <div class="filters">
      <input
        v-model="searchQuery"
        type="text"
        class="search-input"
        placeholder="Rechercher par nom, salon..."
      />
      <select v-model="filterEventType" class="event-select">
        <option value="">Tous les evenements</option>
        <option v-for="t in eventTypes" :key="t" :value="t">
          {{ eventLabel(t) }}
        </option>
      </select>
    </div>

    <div v-if="loading" class="loading">Chargement...</div>

    <div v-else class="audit-list">
      <div v-for="log in filteredLogs" :key="log.id" class="audit-entry">
        <div :class="['event-icon', `icon--${eventVariant(log.event_type)}`]">
          {{ eventIcon(log.event_type) }}
        </div>
        <div class="entry-content">
          <div class="entry-header">
            <AppBadge :label="eventLabel(log.event_type)" :variant="eventVariant(log.event_type)" />
            <span v-if="log.actor_name" class="actor">
              par <strong>{{ log.actor_name }}</strong>
            </span>
            <span v-if="log.target_name" class="target">
              sur <strong>{{ log.target_name }}</strong>
            </span>
            <span v-if="log.channel_name" class="channel">
              dans <strong>{{ log.channel_name }}</strong>
            </span>
            <span class="timestamp">{{ fmt(log.created_at) }}</span>
          </div>
          <div v-if="hasDetails(log.details)" class="entry-details">
            <template v-if="log.details.new_content">
              <span class="detail-label">Nouveau contenu :</span>
              <span class="detail-value">{{ log.details.new_content }}</span>
            </template>
            <template v-if="log.details.old_roles">
              <span class="detail-label">Roles :</span>
              <span class="detail-value">{{ (log.details.old_roles as string[]).length }} → {{ (log.details.new_roles as string[]).length }}</span>
            </template>
            <template v-if="log.details.from_channel || log.details.to_channel">
              <span class="detail-label">Deplacement :</span>
              <span class="detail-value mono">{{ log.details.from_channel }} → {{ log.details.to_channel }}</span>
            </template>
          </div>
        </div>
      </div>

      <div v-if="filteredLogs.length === 0" class="empty">
        Aucun evenement d'audit
      </div>
    </div>
  </div>
</template>

<script lang="ts">
function hasDetails(details: Record<string, unknown>): boolean {
  return Object.keys(details).length > 0;
}
</script>

<style scoped>
.audit h1 {
  margin-bottom: 20px;
}

.filters {
  display: flex;
  gap: 12px;
  margin-bottom: 20px;
}

.search-input {
  flex: 1;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
}

.search-input::placeholder {
  color: var(--text-secondary);
}

.search-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}

.event-select {
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  min-width: 200px;
}

.event-select:focus {
  outline: none;
  border-color: var(--accent);
}

.audit-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.audit-entry {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 18px;
  display: flex;
  gap: 14px;
  align-items: flex-start;
}

.event-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 14px;
  color: white;
  flex-shrink: 0;
}

.icon--danger { background-color: var(--danger); }
.icon--warning { background-color: var(--warning); color: #1a1b2e; }
.icon--success { background-color: var(--success); color: #1a1b2e; }
.icon--info { background-color: var(--info); }
.icon--default { background-color: var(--bg-hover); color: var(--text-secondary); }

.entry-content {
  flex: 1;
  min-width: 0;
}

.entry-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  font-size: 13px;
}

.actor, .target, .channel {
  color: var(--text-secondary);
}

.actor strong, .target strong, .channel strong {
  color: var(--text-primary);
}

.timestamp {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  white-space: nowrap;
}

.entry-details {
  margin-top: 8px;
  padding: 8px 12px;
  background-color: var(--bg-secondary);
  border-radius: 6px;
  font-size: 12px;
  display: flex;
  gap: 6px;
  align-items: center;
}

.detail-label {
  color: var(--text-secondary);
}

.detail-value {
  color: var(--text-primary);
}

.mono {
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.loading, .empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
