<script setup lang="ts">
import { useAuditLogs } from "@/composables/useAuditLogs";
import { useRealtimeRefresh } from "@/composables/useRealtimeRefresh";
import { usePagination } from "@/composables/usePagination";
import { useFormatDate } from "@/composables/useFormatDate";
import AppBadge from "@/components/atoms/AppBadge.vue";
import LoadingState from "@/components/atoms/LoadingState.vue";
import EmptyState from "@/components/atoms/EmptyState.vue";
import PaginationBar from "@/components/molecules/PaginationBar.vue";
import AuditEventDetail from "@/components/molecules/AuditEventDetail.vue";
import { eventVariant, eventLabel, eventIcon } from "@/utils/variants";

const { filteredLogs, loading, fetchLogs } = useAuditLogs();
const { formatShortDateTime: fmt } = useFormatDate();

useRealtimeRefresh(["log_entry_created"], fetchLogs);
const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedLogs } = usePagination(filteredLogs, 30);
</script>

<template>
  <LoadingState v-if="loading" />
  <div v-else class="audit-list">
    <div v-for="log in paginatedLogs" :key="log.id" class="audit-entry">
      <div :class="['event-icon', `icon--${eventVariant(log.event_type)}`]">
        {{ eventIcon(log.event_type) }}
      </div>
      <div class="entry-content">
        <div class="entry-header">
          <AppBadge :label="eventLabel(log.event_type)" :variant="eventVariant(log.event_type)" />
          <span v-if="log.actor_name" class="actor">par <strong>{{ log.actor_name }}</strong></span>
          <span v-if="log.target_name" class="target">sur <strong>{{ log.target_name }}</strong></span>
          <span v-if="log.channel_name" class="channel">dans <strong>{{ log.channel_name }}</strong></span>
          <span class="timestamp">{{ fmt(log.created_at) }}</span>
        </div>
        <AuditEventDetail :details="log.details" />
      </div>
    </div>

    <EmptyState v-if="filteredLogs.length === 0" message="Aucun evenement d'audit" />

    <PaginationBar
      v-if="filteredLogs.length > 0"
      :current-page="currentPage"
      :total-pages="totalPages"
      :total-items="totalItems"
      :per-page="perPage"
      @update:current-page="currentPage = $event"
      @update:per-page="perPage = $event"
    />
  </div>
</template>

<style scoped>
.audit-list { display: flex; flex-direction: column; gap: 8px; }
.audit-entry {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 14px 18px;
  display: flex;
  gap: 14px;
  align-items: flex-start;
}
.event-icon {
  width: 32px; height: 32px;
  border-radius: var(--radius-md);
  display: flex; align-items: center; justify-content: center;
  font-weight: 700; font-size: 14px;
  color: white; flex-shrink: 0;
}
.icon--danger { background-color: var(--danger); }
.icon--warning { background-color: var(--warning); color: #1a1b2e; }
.icon--success { background-color: var(--success); color: #1a1b2e; }
.icon--info { background-color: var(--info); }
.icon--default { background-color: var(--bg-hover); color: var(--text-secondary); }
.entry-content { flex: 1; min-width: 0; }
.entry-content, .entry-content * {
  overflow-wrap: anywhere;
  word-break: break-word;
}
.entry-header {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  font-size: 13px;
}
.actor, .target, .channel { color: var(--text-secondary); }
.actor strong, .target strong, .channel strong { color: var(--text-primary); }
.timestamp {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  white-space: nowrap;
}
@media (max-width: 768px) {
  .audit-entry { padding: 10px 12px; gap: 10px; font-size: 13px; }
  .event-icon { width: 28px; height: 28px; font-size: 13px; }
}
</style>
