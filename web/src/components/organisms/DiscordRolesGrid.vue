<script setup lang="ts">
import { ref } from "vue";
import { discordRolesService } from "@/services/discordRolesService";
import { useDiscordRoles } from "../../composables/useDiscordRoles";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useFormatDate } from "../../composables/useFormatDate";
import type { DiscordRole } from "../../types";
import AppBadge from "../atoms/AppBadge.vue";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import { errMsg } from "@/utils/errMsg";

const { confirm } = useConfirm();
const { error: showError } = useToast();

const emit = defineEmits<{ edit: [role: DiscordRole] }>();

const { formatShortDateTime: fmt } = useFormatDate();
const { selectedGuildId } = useGuildSelector();
const { filteredRoles, fetchRoles } = useDiscordRoles();

const deleting = ref<string | null>(null);

async function deleteRole(roleId: string, roleName: string) {
  if (!selectedGuildId.value) return;
  if (
    !(await confirm({
      title: "Supprimer le rôle",
      message: `Supprimer le rôle "${roleName}" ? Cette action est irréversible.`,
    }))
  )
    return;
  deleting.value = roleId;
  try {
    await discordRolesService.remove(selectedGuildId.value, roleId);
    await fetchRoles();
  } catch (e) {
    showError(`Erreur suppression rôle : ${errMsg(e)}`);
  } finally {
    deleting.value = null;
  }
}

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
  <div class="roles-grid">
    <div
      v-for="role in filteredRoles"
      :key="role.id"
      class="card role-card"
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
        <div v-if="!role.managed" class="role-actions">
          <button class="btn-edit" @click="emit('edit', role)">Modifier</button>
          <button class="btn-delete" :disabled="deleting === role.id" @click="deleteRole(role.id, role.name)">
            {{ deleting === role.id ? '...' : 'Supprimer' }}
          </button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.roles-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 12px;
}

.role-card { transition: border-color var(--transition-base); }
.role-card:hover { border-color: var(--accent); }
.role-card.managed { opacity: 0.7; }

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
.role-info { display: flex; flex-direction: column; gap: 2px; flex: 1; min-width: 0; }
.role-name {
  font-weight: 600;
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.role-id { font-size: 11px; color: var(--text-secondary); font-family: monospace; }
.role-badges { display: flex; gap: 4px; flex-shrink: 0; }

.role-details { display: flex; flex-direction: column; gap: 6px; }
.detail-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 12px;
}
.detail-label { color: var(--text-secondary); }
.detail-value { color: var(--text-primary); }
.detail-value.mono { font-family: "JetBrains Mono", monospace; }
.perms {
  font-size: 11px;
  max-width: 200px;
  text-align: right;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.role-actions {
  display: flex;
  gap: 8px;
  margin-top: 10px;
  padding-top: 10px;
  border-top: 1px solid var(--border);
}
.btn-edit {
  flex: 1;
  text-align: center;
  background: rgba(88, 101, 242, 0.1);
  color: var(--accent);
  border: 1px solid rgba(88, 101, 242, 0.2);
  border-radius: var(--radius-sm);
  padding: 6px 12px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-edit:hover { background: rgba(88, 101, 242, 0.2); }
.btn-delete {
  text-align: center;
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 6px 12px;
  font-size: 12px;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.btn-delete:hover {
  background: rgba(239, 68, 68, 0.1);
  color: var(--danger);
  border-color: rgba(239, 68, 68, 0.3);
}
.btn-delete:disabled { opacity: 0.4; cursor: not-allowed; }

@media (max-width: 768px) {
  .roles-grid { grid-template-columns: 1fr; }
}
</style>
