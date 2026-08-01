<script setup lang="ts">
import IconButton from "../atoms/IconButton.vue";
import { useNotifications } from "../../composables/useNotifications";
import AppBadge from "../atoms/AppBadge.vue";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { notifications, unreadCount, markAsRead, markAllAsRead, closePanel } = useNotifications();

function severityVariant(severity: string): "danger" | "warning" | "info" | "default" {
  switch (severity) {
    case "critical": return "danger";
    case "high": return "warning";
    case "medium": return "info";
    case "low": return "default";
    default: return "default";
  }
}

function typeIcon(type: string): string {
  switch (type) {
    case "raid": return "R";
    case "infraction": return "!";
    case "ticket": return "T";
    case "bot": return "B";
    default: return "N";
  }
}
</script>

<template>
  <div class="card notification-panel">
    <div class="panel-header">
      <h3>Notifications</h3>
      <div class="panel-actions">
        <button v-if="unreadCount > 0" class="mark-all" @click="markAllAsRead">
          Tout marquer comme lu
        </button>
        <IconButton label="Fermer" variant="neutral" @click="closePanel">&times;</IconButton>
      </div>
    </div>

    <div class="panel-list">
      <div
        v-for="notif in notifications"
        :key="notif.id"
        :class="['notif-item', { unread: !notif.read }]"
        @click="markAsRead(notif.id)"
      >
        <div :class="['notif-icon', `icon--${notif.notification_type}`]">
          {{ typeIcon(notif.notification_type) }}
        </div>
        <div class="notif-content">
          <div class="notif-title-row">
            <span class="notif-title">{{ notif.title }}</span>
            <AppBadge :label="notif.severity" :variant="severityVariant(notif.severity)" />
          </div>
          <p class="notif-message">{{ notif.message }}</p>
          <span class="notif-time">{{ fmt(notif.created_at) }}</span>
        </div>
        <span v-if="!notif.read" class="unread-dot"></span>
      </div>

      <div v-if="notifications.length === 0" class="empty">
        Aucune notification
      </div>
    </div>
  </div>
</template>

<style scoped>
.notification-panel {
  position: absolute;
  top: calc(100% + 8px);
  right: 16px;
  width: 380px;
  max-height: 500px;
  padding: 0; /* panel sans padding, chaque notif-item gere le sien */
  box-shadow: var(--shadow-lg);
  display: flex;
  flex-direction: column;
  z-index: 100;
  overflow: hidden;
}

.panel-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
}

.panel-header h3 {
  font-size: 15px;
  font-weight: 600;
}

.panel-actions {
  display: flex;
  align-items: center;
  gap: 8px;
}

.mark-all {
  background: none;
  color: var(--accent);
  font-size: 12px;
  padding: 4px 8px;
}

.mark-all:hover {
  text-decoration: underline;
}



.panel-list {
  overflow-y: auto;
  flex: 1;
}

.notif-item {
  display: flex;
  align-items: flex-start;
  gap: 12px;
  padding: 14px 20px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background-color var(--transition-fast);
  position: relative;
}

.notif-item:last-child {
  border-bottom: none;
}

.notif-item:hover {
  background-color: var(--bg-hover);
}

.notif-item.unread {
  background-color: var(--accent-bg);
  /* accent-bg est 0.15 d'opacite, on l'attenue via un voile semi-transparent */
  background-color: color-mix(in srgb, var(--accent) 5%, transparent);
}

.notif-icon {
  width: 32px;
  height: 32px;
  border-radius: var(--radius-md);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 13px;
  flex-shrink: 0;
  color: white;
}

.icon--raid { background-color: var(--danger); }
.icon--infraction { background-color: var(--warning); }
.icon--ticket { background-color: var(--info); }
.icon--bot { background-color: var(--accent); }

.notif-content {
  flex: 1;
  min-width: 0;
}

.notif-title-row {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}

.notif-title {
  font-size: 13px;
  font-weight: 600;
}

.notif-message {
  font-size: 12px;
  color: var(--text-secondary);
  line-height: 1.4;
  margin-bottom: 4px;
  display: -webkit-box;
  -webkit-line-clamp: 2;
  -webkit-box-orient: vertical;
  overflow: hidden;
}

.notif-time {
  font-size: 11px;
  color: var(--text-secondary);
  opacity: 0.7;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.unread-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background-color: var(--accent);
  flex-shrink: 0;
  margin-top: 4px;
}

.empty {
  padding: 32px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
}
</style>
