<script setup lang="ts">
import { useAuditLogs } from "../../composables/useAuditLogs";
import { usePagination } from "../../composables/usePagination";
import AppBadge from "../atoms/AppBadge.vue";
import LoadingState from "../atoms/LoadingState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import PaginationBar from "../molecules/PaginationBar.vue";
import { eventVariant, eventLabel, eventIcon } from "../../utils/variants";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { filteredLogs, eventTypes, loading, filterEventType, searchQuery } = useAuditLogs();
const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedLogs } = usePagination(filteredLogs, 30);
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

    <LoadingState v-if="loading" />

    <div v-else class="audit-list">
      <div v-for="log in paginatedLogs" :key="log.id" class="audit-entry">
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
            <!-- Message edit : avant / apres -->
            <template v-if="log.details.old_content !== undefined || log.details.new_content !== undefined">
              <div v-if="log.details.old_content" class="detail-block">
                <span class="detail-label">Avant :</span>
                <span class="detail-value detail-old">{{ log.details.old_content }}</span>
              </div>
              <div v-if="log.details.new_content" class="detail-block">
                <span class="detail-label">Apres :</span>
                <span class="detail-value detail-new">{{ log.details.new_content }}</span>
              </div>
            </template>

            <!-- Message delete : contenu supprime -->
            <template v-if="log.details.content && !log.details.new_content">
              <div class="detail-block">
                <span class="detail-label">Contenu :</span>
                <span class="detail-value">{{ log.details.content }}</span>
              </div>
              <div v-if="log.details.author_name" class="detail-block">
                <span class="detail-label">Auteur :</span>
                <span class="detail-value">{{ log.details.author_name }}</span>
              </div>
            </template>

            <!-- Message delete bulk -->
            <template v-if="log.details.count">
              <div class="detail-block">
                <span class="detail-label">Messages supprimes :</span>
                <span class="detail-value">{{ log.details.count }}</span>
              </div>
            </template>

            <!-- Role create / update -->
            <template v-if="log.details.colour || log.details.changes">
              <div v-if="log.details.colour" class="detail-block">
                <span class="detail-label">Couleur :</span>
                <span class="detail-value"><span class="color-dot" :style="{ backgroundColor: String(log.details.colour) }"></span> {{ log.details.colour }}</span>
              </div>
              <div v-if="log.details.position !== undefined" class="detail-block">
                <span class="detail-label">Position :</span>
                <span class="detail-value">{{ log.details.position }}</span>
              </div>
              <div v-if="log.details.mentionable !== undefined" class="detail-block">
                <span class="detail-label">Mentionnable :</span>
                <span class="detail-value">{{ log.details.mentionable ? 'Oui' : 'Non' }}</span>
              </div>
              <div v-if="log.details.hoist !== undefined" class="detail-block">
                <span class="detail-label">Affiche separement :</span>
                <span class="detail-value">{{ log.details.hoist ? 'Oui' : 'Non' }}</span>
              </div>
            </template>

            <!-- Role update changes list -->
            <template v-if="Array.isArray(log.details.changes)">
              <div v-for="(change, i) in (log.details.changes as string[])" :key="i" class="detail-block">
                <span class="detail-value mono">{{ change }}</span>
              </div>
            </template>

            <!-- Permission diff -->
            <div v-if="log.details.permission_diff" class="detail-block">
              <span class="detail-label">Permissions :</span>
              <pre class="detail-pre">{{ log.details.permission_diff }}</pre>
            </div>

            <!-- Channel create -->
            <template v-if="log.details.kind && !log.details.changes">
              <div class="detail-block">
                <span class="detail-label">Type :</span>
                <span class="detail-value">{{ log.details.kind }}</span>
              </div>
            </template>

            <!-- Roles changes (member_role_update) -->
            <template v-if="log.details.old_roles">
              <div class="detail-block">
                <span class="detail-label">Roles :</span>
                <span class="detail-value">{{ (log.details.old_roles as string[]).length }} → {{ (log.details.new_roles as string[]).length }}</span>
              </div>
            </template>

            <!-- Voice move -->
            <template v-if="log.details.from_channel || log.details.to_channel">
              <div class="detail-block">
                <span class="detail-label">Deplacement :</span>
                <span class="detail-value mono">{{ log.details.from_channel }} → {{ log.details.to_channel }}</span>
              </div>
            </template>

            <!-- Avatar change -->
            <template v-if="log.details.old_avatar_url || log.details.new_avatar_url">
              <div class="detail-avatars">
                <div v-if="log.details.old_avatar_url" class="avatar-block">
                  <span class="detail-label">Avant :</span>
                  <img :src="String(log.details.old_avatar_url)" class="avatar-preview" alt="Ancien avatar" />
                </div>
                <div v-if="log.details.old_avatar_url && log.details.new_avatar_url" class="avatar-arrow">→</div>
                <div v-if="log.details.new_avatar_url" class="avatar-block">
                  <span class="detail-label">{{ log.details.old_avatar_url ? 'Apres :' : 'Nouvel avatar :' }}</span>
                  <img :src="String(log.details.new_avatar_url)" class="avatar-preview" alt="Nouvel avatar" />
                </div>
              </div>
            </template>

            <!-- Member join : account age -->
            <template v-if="log.details.account_created_at">
              <div class="detail-block">
                <span class="detail-label">Compte cree le :</span>
                <span class="detail-value">{{ log.details.account_created_at }}</span>
              </div>
            </template>

            <!-- Anomaly -->
            <template v-if="log.details.anomaly_type">
              <div class="detail-block">
                <span class="detail-label">Type :</span>
                <span class="detail-value">{{ log.details.anomaly_type }}</span>
              </div>
              <div class="detail-block">
                <span class="detail-label">Nombre :</span>
                <span class="detail-value">{{ log.details.count }} en {{ log.details.window_secs }}s</span>
              </div>
            </template>
          </div>
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
  padding: 10px 12px;
  background-color: var(--bg-secondary);
  border-radius: 6px;
  font-size: 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.detail-block {
  display: flex;
  gap: 6px;
  align-items: baseline;
}

.detail-label {
  color: var(--text-secondary);
  white-space: nowrap;
  flex-shrink: 0;
}

.detail-value {
  color: var(--text-primary);
  word-break: break-word;
}

.detail-old {
  color: var(--danger);
  text-decoration: line-through;
  opacity: 0.8;
}

.detail-new {
  color: var(--success);
}

.detail-pre {
  margin: 0;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  font-size: 11px;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}

.color-dot {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  vertical-align: middle;
  margin-right: 4px;
}

.mono {
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.detail-avatars {
  display: flex;
  align-items: center;
  gap: 12px;
}

.avatar-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.avatar-preview {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  border: 2px solid var(--border);
  object-fit: cover;
}

.avatar-arrow {
  font-size: 20px;
  color: var(--text-secondary);
  font-weight: 700;
}

.loading, .empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
