<script setup lang="ts">
import { useSecurity } from "../../composables/useSecurity";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { useSearch } from "../../composables/useSearch";
import type { SecurityEvent } from "../../types";
import AppBadge from "../atoms/AppBadge.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import { severityVariant } from "../../utils/variants";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { events, loading, error, fetchEvents } = useSecurity();
useRealtimeRefresh(["security_event"], fetchEvents);
const { search, filtered: filteredEvents } = useSearch<SecurityEvent>(
  events,
  ["event_type", "severity", "description", "created_at", (e) => e.user_ids?.join(" ")],
);

function eventIcon(type: string): string {
  switch (type) {
    case "raid_detected": return "R";
    case "suspicious_account": return "?";
    case "mass_ban": return "!";
    default: return "S";
  }
}
</script>

<template>
  <div class="security">
    <h1>Evenements de securite</h1>

    <input v-model="search" type="text" placeholder="Rechercher dans tous les champs..." class="search-global" />

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchEvents" />
    <LoadingState v-else-if="loading" />

    <div v-else class="events-list">
      <div v-for="event in filteredEvents" :key="event.id" class="event-card">
        <div :class="['event-icon', `icon--${event.severity}`]">
          {{ eventIcon(event.event_type) }}
        </div>
        <div class="event-content">
          <div class="event-header">
            <span class="event-type">{{ event.event_type.replace("_", " ") }}</span>
            <AppBadge :label="event.severity" :variant="severityVariant(event.severity)" />
            <span class="event-time">{{ fmt(event.created_at) }}</span>
          </div>
          <p class="event-description">{{ event.description }}</p>
          <div v-if="event.user_ids?.length > 0" class="event-users">
            <span class="users-label">Utilisateurs concernes :</span>
            <span v-for="uid in event.user_ids" :key="uid" class="user-chip">{{ uid }}</span>
          </div>
        </div>
      </div>

      <EmptyState v-if="filteredEvents.length === 0" message="Aucun evenement de securite" />
    </div>
  </div>
</template>

<style scoped>
.security h1 {
  margin-bottom: 24px;
}

.search-global { width: 100%; padding: 10px 14px; margin-bottom: 12px; border: 1px solid var(--border); border-radius: 8px; background: var(--bg-card); color: var(--text-primary); font-size: 14px; outline: none; }
.search-global:focus { border-color: var(--accent); }
.search-global::placeholder { color: var(--text-secondary); opacity: 0.6; }

.events-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.event-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 20px;
  display: flex;
  gap: 16px;
}

.event-icon {
  width: 40px;
  height: 40px;
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 16px;
  color: white;
  flex-shrink: 0;
}

.icon--critical { background-color: var(--danger); }
.icon--high { background-color: var(--warning); }
.icon--medium { background-color: var(--info); }
.icon--low { background-color: var(--bg-hover); color: var(--text-secondary); }

.event-content {
  flex: 1;
}

.event-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 8px;
}

.event-type {
  font-weight: 600;
  font-size: 14px;
  text-transform: capitalize;
}

.event-time {
  margin-left: auto;
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.event-description {
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
  margin-bottom: 8px;
}

.event-users {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.users-label {
  font-size: 11px;
  color: var(--text-secondary);
}

.user-chip {
  font-size: 11px;
  padding: 2px 8px;
  border-radius: 4px;
  background-color: var(--bg-hover);
  color: var(--text-secondary);
  font-family: monospace;
}

.loading, .empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
