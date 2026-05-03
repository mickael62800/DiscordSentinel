<script setup lang="ts">
import { ref, computed } from "vue";
import { useRbac } from "@/composables/useRbac";
import { useFormatDate } from "@/composables/useFormatDate";
import { useConfirm } from "@/composables/useConfirm";
import LoadingState from "@/components/atoms/LoadingState.vue";
import EmptyState from "@/components/atoms/EmptyState.vue";
import AppBadge from "@/components/atoms/AppBadge.vue";
import { safeImageUrl } from "@/utils/safeUrl";
import type { RbacRole } from "@/types";

const { users, myRole, loading, refresh, grantRole, updateRole, revokeRole } = useRbac();
const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();

const showAddForm = ref(false);
const newUserId = ref("");
const newDisplayName = ref("");
const newRole = ref<RbacRole>("viewer");
const submitting = ref(false);

const ROLES: RbacRole[] = ["owner", "admin", "moderator", "viewer"];

const canEdit = computed(() => myRole.value?.role === "owner");

function resetForm() {
  newUserId.value = "";
  newDisplayName.value = "";
  newRole.value = "viewer";
  showAddForm.value = false;
}

async function submitAdd() {
  if (!newUserId.value.trim()) return;
  submitting.value = true;
  const ok = await grantRole(
    newUserId.value.trim(),
    newRole.value,
    newDisplayName.value.trim() || undefined,
  );
  submitting.value = false;
  if (ok) resetForm();
}

async function onRoleChange(userId: string, newValue: RbacRole) {
  await updateRole(userId, newValue);
}

async function onRevoke(userId: string, displayName: string) {
  const ok = await confirm({ message: `Revoquer le role de ${displayName} (${userId}) ?` });
  if (ok) await revokeRole(userId);
}

type BadgeVariant = "default" | "success" | "error" | "info" | "warning" | "danger";

function roleVariant(role: RbacRole): BadgeVariant {
  switch (role) {
    case "owner": return "danger";
    case "admin": return "warning";
    case "moderator": return "info";
    case "viewer": return "default";
  }
}
</script>

<template>
  <div>
    <section v-if="canEdit" class="add-section">
      <button v-if="!showAddForm" class="btn-primary" @click="showAddForm = true">
        + Ajouter un utilisateur
      </button>
      <form v-else class="add-form" @submit.prevent="submitAdd">
        <input
          v-model="newUserId"
          type="text"
          placeholder="Discord User ID (17-20 chiffres)"
          required
          pattern="\d{17,20}"
        />
        <input
          v-model="newDisplayName"
          type="text"
          placeholder="Nom d'affichage (optionnel)"
        />
        <select v-model="newRole">
          <option v-for="role in ROLES" :key="role" :value="role">{{ role }}</option>
        </select>
        <button type="submit" :disabled="submitting">{{ submitting ? "..." : "Ajouter" }}</button>
        <button type="button" @click="resetForm">Annuler</button>
      </form>
    </section>

    <LoadingState v-if="loading" message="Chargement des roles..." />

    <EmptyState
      v-else-if="users.length === 0"
      icon="\u{1f465}"
      title="Aucun utilisateur avec role"
      message="Ajoutez un premier owner via SQL direct, ou via l'interface si vous etes deja owner."
    />

    <div v-else class="rbac-table-wrap">
      <table class="rbac-table">
        <thead>
          <tr>
            <th>Utilisateur</th>
            <th>Role</th>
            <th class="col-meta">Attribue le</th>
            <th class="col-meta">Par</th>
            <th v-if="canEdit">Actions</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="user in users" :key="user.discord_user_id">
            <td>
              <div class="user-cell">
                <img
                  v-if="safeImageUrl(user.avatar_url)"
                  :src="safeImageUrl(user.avatar_url) ?? ''"
                  :alt="user.display_name"
                  class="avatar"
                />
                <div class="user-cell-text">
                  <strong>{{ user.display_name }}</strong>
                  <div class="user-id">{{ user.discord_user_id }}</div>
                </div>
              </div>
            </td>
            <td>
              <select
                v-if="canEdit"
                :value="user.role"
                @change="onRoleChange(user.discord_user_id, ($event.target as HTMLSelectElement).value as RbacRole)"
              >
                <option v-for="role in ROLES" :key="role" :value="role">{{ role }}</option>
              </select>
              <AppBadge v-else :label="user.role" :variant="roleVariant(user.role)" />
            </td>
            <td class="col-meta">{{ fmt(user.granted_at) }}</td>
            <td class="col-meta">{{ user.granted_by ?? "—" }}</td>
            <td v-if="canEdit">
              <button class="btn-danger" @click="onRevoke(user.discord_user_id, user.display_name)">
                Revoquer
              </button>
            </td>
          </tr>
        </tbody>
      </table>
    </div>

    <button v-if="!loading" class="btn-refresh" @click="refresh">🔄 Actualiser</button>
  </div>
</template>

<style scoped>
.add-section { margin-bottom: 1.5rem; }
.btn-primary {
  padding: 0.5rem 1rem;
  background: #3498db;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
}
.btn-primary:hover { background: #2980b9; }
.add-form {
  display: flex; flex-wrap: wrap; gap: 0.5rem;
  padding: 1rem;
  background: var(--bg-secondary);
  border-radius: 6px;
}
.add-form input, .add-form select {
  padding: 0.5rem;
  border: 1px solid var(--color-border, #444);
  border-radius: 4px;
  background: var(--bg-primary);
  color: var(--color-text, #eee);
}
.add-form input[type="text"] { flex: 1; min-width: 180px; }
.add-form button {
  padding: 0.5rem 1rem;
  border: none; border-radius: 4px;
  cursor: pointer; font-weight: 500;
}
.add-form button[type="submit"] { background: #27ae60; color: white; }
.add-form button[type="button"] { background: var(--bg-card); color: var(--color-text, #eee); }
.rbac-table-wrap { width: 100%; overflow-x: auto; -webkit-overflow-scrolling: touch; }
.rbac-table {
  width: 100%; border-collapse: collapse;
  background: var(--bg-secondary);
  border-radius: 6px; overflow: hidden;
}
.rbac-table th, .rbac-table td {
  padding: 0.75rem 1rem;
  text-align: left;
  border-bottom: 1px solid var(--color-border, #444);
}
.rbac-table th {
  background: var(--bg-card);
  font-weight: 600;
  color: var(--color-text-muted, #aaa);
  font-size: 0.85rem;
  text-transform: uppercase;
}
.user-cell { display: flex; align-items: center; gap: 0.75rem; }
.user-cell-text { min-width: 0; overflow: hidden; }
.user-cell-text strong {
  display: block; overflow: hidden;
  text-overflow: ellipsis; white-space: nowrap;
}
.avatar { width: 32px; height: 32px; border-radius: 50%; }
.user-id {
  font-size: 0.75rem;
  color: var(--color-text-muted, #888);
  font-family: monospace;
}
.btn-danger {
  padding: 0.25rem 0.75rem;
  background: #e74c3c; color: white;
  border: none; border-radius: 4px;
  cursor: pointer; font-size: 0.85rem;
}
.btn-danger:hover { background: #c0392b; }
.btn-refresh {
  margin-top: 1rem;
  padding: 0.5rem 1rem;
  background: var(--bg-card);
  color: var(--color-text, #eee);
  border: 1px solid var(--color-border, #444);
  border-radius: 4px; cursor: pointer;
}
@media (max-width: 768px) {
  .col-meta { display: none; }
}
@media (max-width: 480px) {
  .add-form input[type="text"] { min-width: 0; width: 100%; }
  .add-form { flex-wrap: wrap; gap: 8px; }
  .rbac-table { font-size: 0.85rem; }
  .rbac-table th, .rbac-table td { padding: 0.5rem 0.6rem; }
  .user-id { font-size: 0.7rem; word-break: break-all; }
}
</style>
