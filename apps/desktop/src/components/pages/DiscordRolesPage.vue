<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useDiscordRoles } from "../../composables/useDiscordRoles";
import { useGuildSelector } from "../../composables/useGuildSelector";
import AppBadge from "../atoms/AppBadge.vue";
import AppInput from "../atoms/AppInput.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { selectedGuildId } = useGuildSelector();
const {
  filteredRoles,
  totalRoles,
  loading,
  error,
  search,
  fetchRoles,
} = useDiscordRoles();

// ── Create role ──
const showCreateModal = ref(false);
const newRoleName = ref("");
const newRoleColor = ref("#5865F2");
const creating = ref(false);

async function createRole() {
  if (!selectedGuildId.value || !newRoleName.value.trim()) return;
  creating.value = true;
  try {
    const colorInt = parseInt(newRoleColor.value.replace("#", ""), 16);
    await invoke("create_discord_role", {
      guildId: selectedGuildId.value,
      name: newRoleName.value.trim(),
      color: colorInt,
      permissions: null,
    });
    showCreateModal.value = false;
    newRoleName.value = "";
    newRoleColor.value = "#5865F2";
    await fetchRoles();
  } catch (e) {
    alert("Erreur creation role: " + e);
  } finally {
    creating.value = false;
  }
}

// ── Edit role ──
const editingRole = ref<string | null>(null);
const editName = ref("");
const editColor = ref("#000000");
const editMentionable = ref(false);
const editHoist = ref(false);
const editPerms = ref<Record<string, boolean>>({});
const saving = ref(false);

const PERMISSION_FLAGS: { key: string; label: string; bit: bigint }[] = [
  { key: "admin", label: "Administrateur", bit: 0x8n },
  { key: "manage_guild", label: "Gerer le serveur", bit: 0x20n },
  { key: "manage_roles", label: "Gerer les roles", bit: 0x10000000n },
  { key: "manage_channels", label: "Gerer les salons", bit: 0x10n },
  { key: "kick", label: "Expulser", bit: 0x2n },
  { key: "ban", label: "Bannir", bit: 0x4n },
  { key: "moderate_members", label: "Moderer les membres", bit: 0x10000000000n },
  { key: "manage_messages", label: "Gerer les messages", bit: 0x2000n },
  { key: "mention_everyone", label: "Mentionner @everyone", bit: 0x20000n },
  { key: "send_messages", label: "Envoyer des messages", bit: 0x800n },
  { key: "connect", label: "Se connecter (vocal)", bit: 0x100000n },
  { key: "speak", label: "Parler (vocal)", bit: 0x200000n },
  { key: "mute_members", label: "Muter des membres", bit: 0x400000n },
  { key: "move_members", label: "Deplacer des membres", bit: 0x1000000n },
];

function parsePerms(permsStr: string): Record<string, boolean> {
  const bits = BigInt(permsStr);
  const result: Record<string, boolean> = {};
  for (const flag of PERMISSION_FLAGS) {
    result[flag.key] = (bits & flag.bit) !== 0n;
  }
  return result;
}

function buildPermsString(perms: Record<string, boolean>): string {
  let bits = 0n;
  for (const flag of PERMISSION_FLAGS) {
    if (perms[flag.key]) bits |= flag.bit;
  }
  return bits.toString();
}

function startEdit(role: { id: string; name: string; color: number; mentionable: boolean; permissions: string }) {
  editingRole.value = role.id;
  editName.value = role.name;
  editColor.value = `#${role.color.toString(16).padStart(6, "0")}`;
  editMentionable.value = role.mentionable;
  editHoist.value = false; // Discord API ne retourne pas hoist dans notre sync, on laisse a false
  editPerms.value = parsePerms(role.permissions);
}

function cancelEdit() {
  editingRole.value = null;
}

async function saveEdit() {
  if (!selectedGuildId.value || !editingRole.value) return;
  saving.value = true;
  try {
    const colorInt = parseInt(editColor.value.replace("#", ""), 16);
    const permsStr = buildPermsString(editPerms.value);
    await invoke("edit_discord_role", {
      guildId: selectedGuildId.value,
      roleId: editingRole.value,
      name: editName.value.trim() || null,
      color: colorInt,
      permissions: permsStr,
      mentionable: editMentionable.value,
      hoist: editHoist.value,
    });
    editingRole.value = null;
    await fetchRoles();
  } catch (e) {
    alert("Erreur modification role: " + e);
  } finally {
    saving.value = false;
  }
}

// ── Delete role ──
const deleting = ref<string | null>(null);

async function deleteRole(roleId: string, roleName: string) {
  if (!selectedGuildId.value) return;
  if (!confirm(`Supprimer le role "${roleName}" ? Cette action est irreversible.`)) return;
  deleting.value = roleId;
  try {
    await invoke("delete_discord_role", {
      guildId: selectedGuildId.value,
      roleId,
    });
    await fetchRoles();
  } catch (e) {
    alert("Erreur suppression role: " + e);
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
      <button class="btn-create" @click="showCreateModal = true">+ Creer un role</button>
    </div>

    <!-- Modale creation -->
    <div v-if="showCreateModal" class="modal-overlay" @click.self="showCreateModal = false">
      <div class="modal-content">
        <h3>Creer un role</h3>
        <div class="modal-field">
          <label>Nom</label>
          <input v-model="newRoleName" type="text" class="modal-input" placeholder="Nom du role" />
        </div>
        <div class="modal-field">
          <label>Couleur</label>
          <div class="color-row">
            <input v-model="newRoleColor" type="color" class="color-picker" />
            <span class="color-hex">{{ newRoleColor }}</span>
          </div>
        </div>
        <div class="modal-actions">
          <button class="btn-cancel" @click="showCreateModal = false">Annuler</button>
          <button class="btn-save" :disabled="!newRoleName.trim() || creating" @click="createRole">
            {{ creating ? 'Creation...' : 'Creer' }}
          </button>
        </div>
      </div>
    </div>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchRoles" />
    <LoadingState v-else-if="loading" />

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

        <!-- Mode edition -->
        <div v-if="editingRole === role.id" class="edit-form">
          <div class="edit-section">
            <span class="edit-section-title">General</span>
            <div class="edit-row">
              <input v-model="editName" type="text" class="edit-input" placeholder="Nom" />
              <input v-model="editColor" type="color" class="color-picker" />
            </div>
            <div class="edit-toggles">
              <label class="edit-check">
                <input v-model="editMentionable" type="checkbox" /> Mentionnable
              </label>
              <label class="edit-check">
                <input v-model="editHoist" type="checkbox" /> Affiche separement
              </label>
            </div>
          </div>
          <div class="edit-section">
            <span class="edit-section-title">Permissions</span>
            <div class="perms-grid">
              <label
                v-for="flag in PERMISSION_FLAGS"
                :key="flag.key"
                class="perm-check"
                :class="{ active: editPerms[flag.key] }"
              >
                <input v-model="editPerms[flag.key]" type="checkbox" />
                {{ flag.label }}
              </label>
            </div>
          </div>
          <div class="edit-actions">
            <button class="btn-cancel-sm" @click="cancelEdit">Annuler</button>
            <button class="btn-save-sm" :disabled="saving" @click="saveEdit">
              {{ saving ? '...' : 'Sauver' }}
            </button>
          </div>
        </div>

        <!-- Mode lecture -->
        <div v-else class="role-details">
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
            <button class="btn-edit" @click="startEdit(role)">Modifier</button>
            <button class="btn-delete" :disabled="deleting === role.id" @click="deleteRole(role.id, role.name)">
              {{ deleting === role.id ? '...' : 'Supprimer' }}
            </button>
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

/* Toolbar */
.toolbar { display: flex; gap: 12px; align-items: center; }
.btn-create {
  background: var(--accent); color: white; border: none; border-radius: 8px;
  padding: 10px 20px; font-size: 13px; font-weight: 600; cursor: pointer;
  white-space: nowrap; transition: opacity 0.2s;
}
.btn-create:hover { opacity: 0.85; }

/* Role actions */
.role-actions { display: flex; gap: 8px; margin-top: 8px; justify-content: flex-end; }
.btn-edit {
  background: rgba(88, 101, 242, 0.15); color: var(--accent); border: 1px solid rgba(88, 101, 242, 0.3);
  border-radius: 6px; padding: 4px 12px; font-size: 12px; font-weight: 600; cursor: pointer;
}
.btn-edit:hover { background: rgba(88, 101, 242, 0.3); }
.btn-delete {
  background: rgba(239, 68, 68, 0.15); color: #ef4444; border: 1px solid rgba(239, 68, 68, 0.3);
  border-radius: 6px; padding: 4px 12px; font-size: 12px; font-weight: 600; cursor: pointer;
}
.btn-delete:hover { background: rgba(239, 68, 68, 0.3); }
.btn-delete:disabled { opacity: 0.4; cursor: not-allowed; }

/* Edit form inline */
.edit-form { margin-top: 10px; padding: 12px; background: var(--bg-secondary); border-radius: 8px; }
.edit-row { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
.edit-input {
  flex: 1; min-width: 150px; padding: 8px 10px; background: var(--bg-primary); border: 1px solid var(--border);
  border-radius: 6px; color: var(--text-primary); font-size: 13px;
}
.edit-input:focus { border-color: var(--accent); outline: none; }
.edit-check { font-size: 12px; color: var(--text-secondary); display: flex; align-items: center; gap: 4px; cursor: pointer; }
.edit-actions { display: flex; gap: 8px; margin-top: 8px; justify-content: flex-end; }
.btn-cancel-sm {
  background: transparent; border: 1px solid var(--border); border-radius: 6px;
  padding: 4px 12px; color: var(--text-primary); font-size: 12px; cursor: pointer;
}
.btn-save-sm {
  background: var(--accent); color: white; border: none; border-radius: 6px;
  padding: 4px 16px; font-size: 12px; font-weight: 600; cursor: pointer;
}
.btn-save-sm:disabled { opacity: 0.4; cursor: not-allowed; }

/* Color picker */
.color-picker { width: 36px; height: 36px; border: none; border-radius: 6px; cursor: pointer; padding: 0; }
.color-row { display: flex; align-items: center; gap: 10px; }
.color-hex { font-size: 13px; font-family: monospace; color: var(--text-secondary); }

/* Modal */
.modal-overlay {
  position: fixed; inset: 0; background: rgba(0,0,0,0.6);
  display: flex; align-items: center; justify-content: center; z-index: 1000;
}
.modal-content {
  background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px;
  padding: 24px; width: 100%; max-width: 400px; box-shadow: 0 20px 60px rgba(0,0,0,0.4);
}
.modal-content h3 { margin: 0 0 16px 0; font-size: 18px; }
.modal-field { margin-bottom: 14px; }
.modal-field label { display: block; font-size: 13px; font-weight: 600; color: var(--text-secondary); margin-bottom: 6px; }
.modal-input {
  width: 100%; padding: 10px 12px; background: var(--bg-primary); border: 1px solid var(--border);
  border-radius: 8px; color: var(--text-primary); font-size: 14px;
}
.modal-input:focus { border-color: var(--accent); outline: none; }
.modal-actions { display: flex; gap: 10px; justify-content: flex-end; margin-top: 16px; }
.btn-cancel {
  background: transparent; border: 1px solid var(--border); border-radius: 6px;
  padding: 8px 16px; color: var(--text-primary); font-size: 13px; cursor: pointer;
}
.btn-save {
  background: var(--accent); color: white; border: none; border-radius: 6px;
  padding: 8px 20px; font-size: 13px; font-weight: 600; cursor: pointer;
}
.btn-save:disabled { opacity: 0.5; cursor: not-allowed; }

/* Edit sections */
.edit-section { margin-bottom: 12px; }
.edit-section-title {
  font-size: 11px; font-weight: 600; color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: 0.5px; display: block; margin-bottom: 8px;
}
.edit-toggles { display: flex; gap: 16px; margin-top: 8px; }

/* Permissions grid */
.perms-grid {
  display: grid; grid-template-columns: repeat(3, 1fr); gap: 6px;
}
.perm-check {
  font-size: 12px; color: var(--text-secondary); display: flex; align-items: center; gap: 4px;
  cursor: pointer; padding: 4px 8px; border-radius: 4px; transition: all 0.15s;
  border: 1px solid transparent;
}
.perm-check:hover { background: var(--bg-hover); }
.perm-check.active {
  color: var(--accent); background: rgba(88, 101, 242, 0.08);
  border-color: rgba(88, 101, 242, 0.2);
}
.perm-check input[type="checkbox"] { accent-color: var(--accent); }
</style>
