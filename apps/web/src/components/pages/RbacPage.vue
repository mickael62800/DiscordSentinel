<script setup lang="ts">
import { ref, computed } from "vue";
import { useRbac } from "../../composables/useRbac";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import LoadingState from "../atoms/LoadingState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import AppBadge from "../atoms/AppBadge.vue";
import ComponentVisibilityGrid from "../organisms/ComponentVisibilityGrid.vue";
import InvitationsManager from "../organisms/InvitationsManager.vue";
import { safeImageUrl } from "../../utils/safeUrl";
import type { RbacRole } from "../../types";

const { users, myRole, loading, refresh, grantRole, updateRole, revokeRole } = useRbac();
const { selectedGuild } = useGuildSelector();
const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();

// Formulaire inline d'ajout d'un nouveau user
const showAddForm = ref(false);
const newUserId = ref("");
const newDisplayName = ref("");
const newRole = ref<RbacRole>("viewer");
const submitting = ref(false);

const ROLES: RbacRole[] = ["owner", "admin", "moderator", "viewer"];

// Gate d'affichage : seul un owner peut modifier. Admin peut lister.
const canEdit = computed(() => myRole.value?.role === "owner");
const canView = computed(() => {
  const r = myRole.value?.role;
  return r === "owner" || r === "admin";
});

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
  const ok = await confirm({
    message: `Revoquer le role de ${displayName} (${userId}) ?`,
  });
  if (ok) await revokeRole(userId);
}

type BadgeVariant = "default" | "success" | "error" | "info" | "warning" | "danger";

function roleVariant(role: RbacRole): BadgeVariant {
  switch (role) {
    case "owner":
      return "danger";
    case "admin":
      return "warning";
    case "moderator":
      return "info";
    case "viewer":
      return "default";
  }
}
</script>

<template>
  <div class="rbac-page">
    <header class="page-header">
      <h1>\u{1f510} Gestion RBAC</h1>
      <p class="subtitle">
        Gerez les roles applicatifs des utilisateurs pour
        <strong>{{ selectedGuild?.name ?? "cette guild" }}</strong>
      </p>
      <div v-if="myRole" class="my-role">
        Votre role :
        <AppBadge :label="myRole.role" :variant="roleVariant(myRole.role)" />
      </div>
    </header>

    <div v-if="!selectedGuild" class="no-guild">
      Selectionnez une guild dans la barre laterale pour gerer les acces.
    </div>

    <template v-else-if="!canView">
      <EmptyState
        icon="\u{1f512}"
        title="Acces refuse"
        message="Vous avez besoin du role admin ou owner pour voir la gestion RBAC."
      />
    </template>

    <template v-else>
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
            <option v-for="role in ROLES" :key="role" :value="role">
              {{ role }}
            </option>
          </select>
          <button type="submit" :disabled="submitting">
            {{ submitting ? "..." : "Ajouter" }}
          </button>
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

      <table v-else class="rbac-table">
        <thead>
          <tr>
            <th>Utilisateur</th>
            <th>Role</th>
            <th>Attribue le</th>
            <th>Par</th>
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
                <div>
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
                <option v-for="role in ROLES" :key="role" :value="role">
                  {{ role }}
                </option>
              </select>
              <AppBadge v-else :label="user.role" :variant="roleVariant(user.role)" />
            </td>
            <td>{{ fmt(user.granted_at) }}</td>
            <td>{{ user.granted_by ?? "—" }}</td>
            <td v-if="canEdit">
              <button
                class="btn-danger"
                @click="onRevoke(user.discord_user_id, user.display_name)"
              >
                Revoquer
              </button>
            </td>
          </tr>
        </tbody>
      </table>

      <button v-if="canView && !loading" class="btn-refresh" @click="refresh">
        \u{1f504} Actualiser
      </button>

      <!-- Codes d'invitation (owner only) -->
      <InvitationsManager v-if="canEdit" />

      <!-- Visibilite des composants par role (owner only) -->
      <ComponentVisibilityGrid v-if="canEdit" />
    </template>
  </div>
</template>

<style scoped>
.rbac-page {
  padding: 1.5rem;
  max-width: 1200px;
}

.page-header h1 {
  margin: 0 0 0.25rem 0;
}

.subtitle {
  color: var(--color-text-muted, #888);
  margin: 0.25rem 0 1rem 0;
}

.my-role {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin-bottom: 1.5rem;
  font-size: 0.9rem;
}

.no-guild {
  padding: 2rem;
  text-align: center;
  color: var(--color-text-muted, #888);
  font-style: italic;
}

.add-section {
  margin-bottom: 1.5rem;
}

.btn-primary {
  padding: 0.5rem 1rem;
  background: #3498db;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
}

.btn-primary:hover {
  background: #2980b9;
}

.add-form {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
  padding: 1rem;
  background: var(--bg-secondary);
  border-radius: 6px;
}

.add-form input,
.add-form select {
  padding: 0.5rem;
  border: 1px solid var(--color-border, #444);
  border-radius: 4px;
  background: var(--bg-primary);
  color: var(--color-text, #eee);
}

.add-form input[type="text"] {
  flex: 1;
  min-width: 180px;
}

.add-form button {
  padding: 0.5rem 1rem;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-weight: 500;
}

.add-form button[type="submit"] {
  background: #27ae60;
  color: white;
}

.add-form button[type="button"] {
  background: var(--bg-card);
  color: var(--color-text, #eee);
}

.rbac-table {
  width: 100%;
  border-collapse: collapse;
  background: var(--bg-secondary);
  border-radius: 6px;
  overflow: hidden;
}

.rbac-table th,
.rbac-table td {
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

.user-cell {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
}

.user-id {
  font-size: 0.75rem;
  color: var(--color-text-muted, #888);
  font-family: monospace;
}

.btn-danger {
  padding: 0.25rem 0.75rem;
  background: #e74c3c;
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 0.85rem;
}

.btn-danger:hover {
  background: #c0392b;
}

.btn-refresh {
  margin-top: 1rem;
  padding: 0.5rem 1rem;
  background: var(--bg-card);
  color: var(--color-text, #eee);
  border: 1px solid var(--color-border, #444);
  border-radius: 4px;
  cursor: pointer;
}
</style>
