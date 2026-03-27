<script setup lang="ts">
import { useSecurity } from "../../composables/useSecurity";
import AppBadge from "../atoms/AppBadge.vue";

const { events, loading } = useSecurity();

function severityVariant(severity: string): "danger" | "warning" | "info" | "default" {
  switch (severity) {
    case "critical": return "danger";
    case "high": return "warning";
    case "medium": return "info";
    default: return "default";
  }
}

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

    <div v-if="loading" class="loading">Chargement...</div>

    <div v-else class="events-list">
      <div v-for="event in events" :key="event.id" class="event-card">
        <div :class="['event-icon', `icon--${event.severity}`]">
          {{ eventIcon(event.event_type) }}
        </div>
        <div class="event-content">
          <div class="event-header">
            <span class="event-type">{{ event.event_type.replace("_", " ") }}</span>
            <AppBadge :label="event.severity" :variant="severityVariant(event.severity)" />
            <span class="event-time">{{ event.created_at }}</span>
          </div>
          <p class="event-description">{{ event.description }}</p>
          <div v-if="event.user_ids.length > 0" class="event-users">
            <span class="users-label">Utilisateurs concernes :</span>
            <span v-for="uid in event.user_ids" :key="uid" class="user-chip">{{ uid }}</span>
          </div>
        </div>
      </div>

      <div v-if="events.length === 0" class="empty">Aucun evenement de securite</div>
    </div>
  </div>
</template>

<style scoped>
.security h1 {
  margin-bottom: 24px;
}

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
