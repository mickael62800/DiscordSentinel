<script setup lang="ts">
import { useFormatDate } from "../../composables/useFormatDate";
import type { UserActivity } from "../../types";

defineProps<{
  activities: UserActivity[];
  loading: boolean;
}>();

const { formatShortDateTime: fmt } = useFormatDate();

function eventIcon(type: string): string {
  switch (type) {
    case "message_sent": return "\u{1F4AC}";
    case "message_edited": return "\u270F\uFE0F";
    case "message_deleted": return "\u{1F5D1}\uFE0F";
    case "voice_join": return "\u{1F50A}";
    case "voice_leave": return "\u{1F507}";
    case "nickname_changed": return "\u{1F4DD}";
    case "role_added": return "\u{1F3F7}\uFE0F";
    case "role_removed": return "\u{1F3F7}\uFE0F";
    case "member_join": return "\u27A1\uFE0F";
    case "member_leave": return "\u2B05\uFE0F";
    case "avatar_changed": return "\u{1F5BC}\uFE0F";
    case "roles_changed": return "\u{1F3F7}\uFE0F";
    case "voice_move": return "\u{1F500}";
    default: return "\u{1F4CB}";
  }
}

function eventLabel(type: string): string {
  switch (type) {
    case "message_sent": return "Message envoye";
    case "message_edited": return "Message edite";
    case "message_deleted": return "Message supprime";
    case "voice_join": return "Rejoint vocal";
    case "voice_leave": return "Quitte vocal";
    case "nickname_changed": return "Pseudo change";
    case "role_added": return "Role ajoute";
    case "role_removed": return "Role retire";
    case "member_join": return "A rejoint le serveur";
    case "member_leave": return "A quitte le serveur";
    case "avatar_changed": return "Avatar modifie";
    case "roles_changed": return "Roles modifies";
    case "voice_move": return "Change de salon vocal";
    default: return type;
  }
}
</script>

<template>
  <div v-if="loading" class="loading">Chargement...</div>
  <div v-else-if="activities.length === 0" class="empty-timeline">
    Aucune activite enregistree pour cet utilisateur.
  </div>
  <div v-else class="timeline">
    <div v-for="act in activities" :key="act.id" class="timeline-item">
      <span class="timeline-icon">{{ eventIcon(act.event_type) }}</span>
      <div class="timeline-content">
        <div class="timeline-header">
          <span class="timeline-label">{{ eventLabel(act.event_type) }}</span>
          <span v-if="act.channel_name" class="timeline-channel">#{{ act.channel_name }}</span>
          <span class="timeline-date">{{ fmt(act.created_at) }}</span>
        </div>
        <div v-if="act.content" class="timeline-text">{{ act.content }}</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.timeline {
  display: flex;
  flex-direction: column;
  gap: 2px;
  max-height: 400px;
  overflow-y: auto;
}

.timeline-item {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 8px 12px;
  border-radius: 6px;
  transition: background var(--transition-fast);
}

.timeline-item:hover {
  background: var(--bg-hover);
}

.timeline-icon {
  font-size: 16px;
  flex-shrink: 0;
  margin-top: 2px;
}

.timeline-content {
  flex: 1;
  min-width: 0;
}

.timeline-header {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 12px;
}

.timeline-label {
  font-weight: 600;
  color: var(--text-primary);
}

.timeline-channel {
  color: var(--accent);
  font-size: 11px;
}

.timeline-date {
  color: var(--text-secondary);
  font-size: 11px;
  margin-left: auto;
  white-space: nowrap;
}

.timeline-text {
  font-size: 12px;
  color: var(--text-secondary);
  margin-top: 2px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.empty-timeline {
  color: var(--text-secondary);
  font-size: 13px;
  padding: 16px;
  text-align: center;
}

.loading {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
