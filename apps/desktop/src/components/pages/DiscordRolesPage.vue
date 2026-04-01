<script setup lang="ts">
import { useDiscordRoles } from "../../composables/useDiscordRoles";
import AppBadge from "../atoms/AppBadge.vue";
import AppInput from "../atoms/AppInput.vue";
import LoadingState from "../atoms/LoadingState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const {
  filteredRoles,
  totalRoles,
  loading,
  search,
} = useDiscordRoles();

function colorHex(color: number): string {
  if (color === 0) return "var(--text-secondary)";
  return `#${color.toString(16).padStart(6, "0")}`;
}

function formatPermissions(perms: string): string {
  const bits = BigInt(perms);
  const flags: string[] = [];
  if (bits & 0x8n) flags.push("Admin");
  if (bits & 0x4n) flags.push("Ban");
  if (bits & 0x2n) flags.push("Kick");
  if (bits & 0x10n) flags.push("Channels");
  if (bits & 0x20n) flags.push("Server");
  if (bits & 0x2000n) flags.push("Messages");
  if (bits & 0x800000n) flags.push("Mention @all");
  if (flags.length === 0) return "Aucune permission notable";
  return flags.join(", ");
}
</script>

<template>
  <div class="discord-roles">
    <div class="header">
      <h1>Roles Discord</h1>
      <span class="role-count">{{ totalRoles }} roles</span>
      <router-link to="/role-panels" class="cross-link">&larr; Panels de roles</router-link>
    </div>

    <div class="toolbar">
      <AppInput
        v-model="search"
        placeholder="Rechercher un role..."
      />
    </div>

    <LoadingState v-if="loading" />

    <div v-else-if="filteredRoles.length === 0">
      <EmptyState message="Aucun role trouve" />
    </div>

    <div v-else class="roles-grid">
      <div
        v-for="role in filteredRoles"
        :key="role.id"
        class="role-card"
        :class="{ managed: role.managed }"
      >
        <div class="role-header">
          <div class="role-color" :style="{ backgroundColor: colorHex(role.color) }"></div>
          <div class="role-info">
            <span class="role-name" :style="{ color: colorHex(role.color) }">{{ role.name }}</span>
            <span class="role-id">{{ role.id }}</span>
          </div>
          <div class="role-badges">
            <AppBadge v-if="role.managed" label="Bot" variant="info" />
            <AppBadge v-if="role.mentionable" label="Mentionnable" variant="warning" />
            <AppBadge v-if="formatPermissions(role.permissions).includes('Admin')" label="Admin" variant="danger" />
          </div>
        </div>

        <div class="role-details">
          <div class="detail-row">
            <span class="detail-label">Position</span>
            <span class="detail-value">{{ role.position }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Permissions</span>
            <span class="detail-value perms">{{ formatPermissions(role.permissions) }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Synchro</span>
            <span class="detail-value mono">{{ fmt(role.synced_at) }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.discord-roles h1 { margin: 0; }

.header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}

.cross-link { margin-left: auto; font-size: 13px; font-weight: 600; color: var(--accent); text-decoration: none; padding: 8px 16px; border: 1px solid var(--accent); border-radius: 8px; white-space: nowrap; transition: all 0.15s; }
.cross-link:hover { background: var(--accent); color: white; }

.role-count {
  font-size: 13px;
  color: var(--text-secondary);
  background: var(--bg-card);
  padding: 4px 10px;
  border-radius: 12px;
}

.toolbar {
  display: flex;
  margin-bottom: 16px;
}

.toolbar input {
  max-width: 360px;
}

.roles-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 12px;
}

.role-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
  transition: border-color 0.2s;
}

.role-card:hover {
  border-color: var(--accent);
}

.role-card.managed {
  opacity: 0.7;
}

.role-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.role-color {
  width: 16px;
  height: 16px;
  border-radius: 50%;
  flex-shrink: 0;
  border: 2px solid var(--border);
}

.role-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  flex: 1;
  min-width: 0;
}

.role-name {
  font-weight: 600;
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.role-id {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: monospace;
}

.role-badges {
  display: flex;
  gap: 4px;
  flex-shrink: 0;
}

.role-details {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.detail-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
}

.detail-label {
  color: var(--text-secondary);
}

.detail-value {
  color: var(--text-primary);
}

.perms {
  font-size: 11px;
  max-width: 200px;
  text-align: right;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
</style>
