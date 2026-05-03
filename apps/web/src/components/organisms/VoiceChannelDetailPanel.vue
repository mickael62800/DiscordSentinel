<script setup lang="ts">
import type { VoiceChannelDetail } from "../../types";
import type { VoiceChannelEvent } from "@/services/voiceChannelsService";
import { useFormatDate } from "../../composables/useFormatDate";
import AppBadge from "../atoms/AppBadge.vue";

defineProps<{
  detail: VoiceChannelDetail | null;
  events: VoiceChannelEvent[];
  detailLoading: boolean;
  eventsLoading: boolean;
}>();

const emit = defineEmits<{ back: [] }>();

const { formatShortDateTime: fmt } = useFormatDate();

function eventLabel(type: string): string {
  switch (type) {
    case "voice_join": return "Entree";
    case "voice_leave": return "Sortie";
    case "voice_move": return "Deplacement";
    case "voice_channel_created": return "Salon cree";
    case "voice_channel_updated": return "Modification";
    case "voice_channel_closed": return "Salon ferme";
    default: return type;
  }
}

function eventVariant(type: string): "info" | "warning" | "danger" | "default" {
  switch (type) {
    case "voice_join":
    case "voice_channel_created": return "info";
    case "voice_leave":
    case "voice_move": return "default";
    case "voice_channel_updated": return "warning";
    case "voice_channel_closed": return "danger";
    default: return "default";
  }
}

function formatEventDetails(type: string, details: Record<string, unknown>): string {
  if (!details || Object.keys(details).length === 0) return "";
  if (type === "voice_move") {
    const from = details.from_channel ?? "?";
    const to = details.to_channel ?? "?";
    return `${from} -> ${to}`;
  }
  const entries = Object.entries(details).filter(([, v]) => v !== null && v !== undefined);
  if (entries.length === 0) return "";
  return entries.map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(", ");
}

function kindVariant(kind: string): "info" | "warning" | "default" {
  switch (kind) {
    case "public": return "info";
    case "private": return "warning";
    default: return "default";
  }
}
</script>

<template>
  <div class="detail-view">
    <button class="back-btn" @click="emit('back')">&larr; Retour</button>

    <div v-if="detailLoading" class="loading">Chargement...</div>
    <div v-else-if="detail" class="detail-content">
      <h2>{{ detail.channel.channel_name }}</h2>
      <div class="detail-grid">
        <div><strong>Proprietaire :</strong> {{ detail.channel.owner_name }}</div>
        <div>
          <strong>Type :</strong>
          <AppBadge :label="detail.channel.kind" :variant="kindVariant(detail.channel.kind)" />
        </div>
        <div><strong>Visibilite :</strong> {{ detail.channel.visibility }}</div>
        <div><strong>Verrouille :</strong> {{ detail.channel.locked ? 'Oui' : 'Non' }}</div>
        <div><strong>File d'attente :</strong> {{ detail.channel.queue_enabled ? 'Active' : 'Desactive' }}</div>
        <div v-if="detail.channel.member_limit"><strong>Limite :</strong> {{ detail.channel.member_limit }}</div>
        <div v-if="detail.channel.status"><strong>Statut :</strong> {{ detail.channel.status }}</div>
        <div><strong>Cree le :</strong> {{ fmt(detail.channel.created_at) }}</div>
      </div>

      <h3 v-if="detail.co_admins.length">Co-admins ({{ detail.co_admins.length }})</h3>
      <ul v-if="detail.co_admins.length" class="admin-list">
        <li v-for="ca in detail.co_admins" :key="ca.id">{{ ca.user_name }}</li>
      </ul>

      <h3>Timeline</h3>
      <div v-if="eventsLoading" class="loading">Chargement de la timeline...</div>
      <div v-else-if="events.length === 0" class="empty">Aucun evenement enregistre pour ce salon</div>
      <ul v-else class="timeline">
        <li v-for="ev in events" :key="ev.id" class="timeline-item">
          <span class="timeline-time">{{ fmt(ev.created_at) }}</span>
          <AppBadge :label="eventLabel(ev.event_type)" :variant="eventVariant(ev.event_type)" />
          <span v-if="ev.actor_name" class="timeline-actor">{{ ev.actor_name }}</span>
          <span class="timeline-details">{{ formatEventDetails(ev.event_type, ev.details) }}</span>
        </li>
      </ul>

      <h3 v-if="detail.bans.length">Bans ({{ detail.bans.length }})</h3>
      <div v-if="detail.bans.length" class="bans-table">
        <div v-for="ban in detail.bans" :key="ban.id" class="ban-row">
          <span>{{ ban.user_name }}</span>
          <span>par {{ ban.banned_by }}</span>
          <span v-if="ban.reason">{{ ban.reason }}</span>
          <span v-if="ban.expires_at">Expire : {{ new Date(ban.expires_at).toLocaleString() }}</span>
          <AppBadge v-else label="Permanent" variant="danger" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.back-btn {
  background: none;
  color: var(--accent);
  font-size: 14px;
  margin-bottom: 16px;
  cursor: pointer;
  border: none;
  padding: 0;
}
.back-btn:hover { text-decoration: underline; }

.detail-content h2 { font-size: 20px; margin-bottom: 16px; }
.detail-content h3 { font-size: 16px; margin-bottom: 8px; margin-top: 16px; }

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px 24px;
  margin-bottom: 24px;
}

.admin-list { list-style: disc; padding-left: 20px; }
.admin-list li { padding: 4px 0; }

.ban-row {
  display: flex;
  gap: 16px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
  align-items: center;
  font-size: 13px;
}

.timeline {
  list-style: none;
  padding: 0;
  margin: 0 0 24px;
  border-left: 2px solid var(--border);
}
.timeline-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  font-size: 13px;
  border-bottom: 1px solid var(--border);
}
.timeline-item:last-child { border-bottom: none; }
.timeline-time {
  color: var(--text-secondary);
  font-size: 11px;
  min-width: 130px;
  font-variant-numeric: tabular-nums;
}
.timeline-actor { font-weight: 600; color: var(--text-primary); }
.timeline-details {
  color: var(--text-secondary);
  font-size: 12px;
  font-family: monospace;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.loading, .empty {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary);
}
</style>
