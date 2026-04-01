<script setup lang="ts">
import { computed, ref } from "vue";
import { useWatchedUsers } from "../../composables/useWatchedUsers";
import { usePagination } from "../../composables/usePagination";
import { useGuildMembers } from "../../composables/useGuildMembers";
import { useGuildSelector } from "../../composables/useGuildSelector";
import AppBadge from "../atoms/AppBadge.vue";
import DataTable from "../organisms/DataTable.vue";
import PaginationBar from "../molecules/PaginationBar.vue";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
import type { TableColumn, WatchedUser, GuildMember } from "../../types";
import { actionVariant, severityVariant } from "../../utils/variants";

const { selectedGuildId } = useGuildSelector();
const { searchMembers } = useGuildMembers();

const {
  users,
  loading,
  searchQuery,
  riskFilter,
  selectedUser,
  dossier,
  dossierLoading,
  selectUser,
  fetchUsers,
} = useWatchedUsers();

// Modale ajout surveillance
const addModalVisible = ref(false);
const addSearch = ref("");
const addSuggestions = ref<GuildMember[]>([]);
const showAddSuggestions = ref(false);
const addSelectedMember = ref<GuildMember | null>(null);
const addReason = ref("");
const addLoading = ref(false);

function openAddModal() {
  addModalVisible.value = true;
  addSearch.value = "";
  addSelectedMember.value = null;
  addReason.value = "";
}

function closeAddModal() {
  addModalVisible.value = false;
}

function onAddSearchInput() {
  addSuggestions.value = searchMembers(addSearch.value);
  showAddSuggestions.value = addSuggestions.value.length > 0;
}

function selectAddMember(member: GuildMember) {
  addSelectedMember.value = member;
  addSearch.value = member.display_name || member.username;
  showAddSuggestions.value = false;
}

function onAddSearchBlur() {
  setTimeout(() => { showAddSuggestions.value = false; }, 200);
}

async function confirmAddWatch() {
  if (!addSelectedMember.value || !selectedGuildId.value) return;
  addLoading.value = true;
  try {
    await fetch("http://localhost:3000/api/watched-users", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        guild_id: selectedGuildId.value,
        user_id: addSelectedMember.value.id,
        username: addSelectedMember.value.display_name || addSelectedMember.value.username,
        reason: addReason.value,
      }),
    });
    closeAddModal();
    if (fetchUsers) await fetchUsers();
  } catch (e) {
    console.error("Erreur ajout surveillance:", e);
  } finally {
    addLoading.value = false;
  }
}

const filteredUsers = computed(() => {
  let list = users.value;
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase();
    list = list.filter((u) =>
      [u.username, u.user_id, u.risk_level, u.guild_id, u.guild_name]
        .some((field) => field?.toLowerCase().includes(q)),
    );
  }
  if (riskFilter.value) {
    list = list.filter((u) => u.risk_level === riskFilter.value);
  }
  return list;
});

const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedUsers } = usePagination(filteredUsers);

function riskLabel(level: string): string {
  switch (level) {
    case "critical": return "Critique";
    case "high": return "Eleve";
    case "medium": return "Moyen";
    case "low": return "Faible";
    default: return level;
  }
}

function totalInfractions(user: WatchedUser): number {
  return user.total_warns + user.total_mutes + user.total_bans;
}

function conductPercent(user: WatchedUser): number | null {
  if (user.conduct_points === null || user.max_conduct_points === null) return null;
  return Math.round((user.conduct_points / user.max_conduct_points) * 100);
}

const dossierInfractionColumns: TableColumn[] = [
  { key: "infraction_type", label: "Type" },
  { key: "reason", label: "Raison" },
  { key: "moderator", label: "Moderateur" },
  { key: "created_at", label: "Date" },
];

const dossierActionColumns: TableColumn[] = [
  { key: "action_type", label: "Action" },
  { key: "reason", label: "Raison" },
  { key: "target_name", label: "Cible" },
];

const dossierConductColumns: TableColumn[] = [
  { key: "delta", label: "Points" },
  { key: "reason", label: "Raison" },
  { key: "points_after", label: "Apres" },
  { key: "created_at", label: "Date" },
];
</script>

<template>
  <div class="watched-users">
    <div class="page-header-row">
      <h1>Surveillance des utilisateurs</h1>
      <button class="add-watch-btn" @click="openAddModal">+ Surveiller un membre</button>
    </div>

    <!-- Filtres -->
    <div class="filters">
      <input
        v-model="searchQuery"
        type="text"
        class="search-input"
        placeholder="Rechercher par nom ou ID..."
      />
      <select v-model="riskFilter" class="risk-select">
        <option value="">Tous les niveaux</option>
        <option value="critical">Critique</option>
        <option value="high">Eleve</option>
        <option value="medium">Moyen</option>
        <option value="low">Faible</option>
      </select>
    </div>

    <div v-if="loading" class="loading">Chargement...</div>

    <div v-else class="content-layout">
      <!-- Liste des utilisateurs -->
      <div class="users-list">
        <div
          v-for="user in paginatedUsers"
          :key="`${user.guild_id}-${user.user_id}`"
          :class="['user-card', { selected: selectedUser?.user_id === user.user_id && selectedUser?.guild_id === user.guild_id }]"
          @click="selectUser(user)"
        >
          <div class="user-card-header">
            <div class="user-identity">
              <div class="user-avatar-placeholder">{{ user.username.charAt(0).toUpperCase() }}</div>
              <div class="user-names">
                <span class="user-name">{{ user.username }}</span>
                <span class="user-id">{{ user.user_id }}</span>
              </div>
            </div>
            <AppBadge :label="riskLabel(user.risk_level)" :variant="severityVariant(user.risk_level)" />
          </div>

          <div class="user-card-stats">
            <div class="stat-item">
              <span class="stat-value stat-warn">{{ user.total_warns }}</span>
              <span class="stat-label">Warns</span>
            </div>
            <div class="stat-item">
              <span class="stat-value stat-mute">{{ user.total_mutes }}</span>
              <span class="stat-label">Mutes</span>
            </div>
            <div class="stat-item">
              <span class="stat-value stat-ban">{{ user.total_bans }}</span>
              <span class="stat-label">Bans</span>
            </div>
            <div class="stat-item">
              <span class="stat-value">{{ totalInfractions(user) }}</span>
              <span class="stat-label">Total</span>
            </div>
          </div>

          <div class="user-card-footer">
            <span v-if="user.last_incident_at" class="last-incident">
              Dernier incident : <span class="mono">{{ user.last_incident_at }}</span>
            </span>
            <span v-if="conductPercent(user) !== null" class="conduct-info">
              Conduite : {{ conductPercent(user) }}%
            </span>
            <span v-if="user.security_events_count > 0" class="security-count">
              {{ user.security_events_count }} evt securite
            </span>
          </div>
        </div>

        <div v-if="filteredUsers.length === 0" class="empty">
          Aucun utilisateur surveille
        </div>

        <PaginationBar
          :current-page="currentPage"
          :total-pages="totalPages"
          :total-items="totalItems"
          :per-page="perPage"
          @update:current-page="currentPage = $event"
          @update:per-page="perPage = $event"
        />
      </div>

      <!-- Panneau dossier -->
      <div v-if="selectedUser" class="dossier-panel">
        <div class="dossier-header">
          <div class="dossier-title">
            <h2>Dossier : {{ selectedUser.username }}</h2>
            <AppBadge :label="riskLabel(selectedUser.risk_level)" :variant="severityVariant(selectedUser.risk_level)" />
          </div>
          <button class="close-btn" @click="selectUser(null)">&times;</button>
        </div>

        <div class="dossier-summary">
          <div class="summary-card">
            <span class="summary-value">{{ selectedUser.user_id }}</span>
            <span class="summary-label">ID Discord</span>
          </div>
          <div class="summary-card">
            <span class="summary-value">{{ selectedUser.guild_name }}</span>
            <span class="summary-label">Serveur</span>
          </div>
          <div class="summary-card">
            <span class="summary-value">{{ totalInfractions(selectedUser) }}</span>
            <span class="summary-label">Infractions</span>
          </div>
          <div class="summary-card">
            <span class="summary-value">{{ selectedUser.security_events_count }}</span>
            <span class="summary-label">Evt Securite</span>
          </div>
          <div v-if="conductPercent(selectedUser) !== null" class="summary-card">
            <span :class="['summary-value', { 'conduct-low': (conductPercent(selectedUser) ?? 0) < 30 }]">
              {{ selectedUser.conduct_points }} / {{ selectedUser.max_conduct_points }}
            </span>
            <span class="summary-label">Points de conduite</span>
          </div>
        </div>

        <div v-if="dossierLoading" class="loading">Chargement du dossier...</div>

        <template v-else-if="dossier">
          <!-- Infractions -->
          <section class="dossier-section">
            <h3>Infractions ({{ dossier.infractions.length }})</h3>
            <DataTable
              :columns="dossierInfractionColumns"
              :rows="(dossier.infractions as unknown as Record<string, unknown>[])"
              empty-message="Aucune infraction"
            >
              <template #cell-infraction_type="{ value }">
                <AppBadge :label="String(value)" :variant="actionVariant(String(value))" />
              </template>
              <template #cell-created_at="{ value }">
                <span class="mono">{{ fmt(String(value)) }}</span>
              </template>
            </DataTable>
          </section>

          <!-- Actions de moderation -->
          <section class="dossier-section">
            <h3>Actions de moderation ({{ dossier.moderation_actions.length }})</h3>
            <DataTable
              :columns="dossierActionColumns"
              :rows="(dossier.moderation_actions as unknown as Record<string, unknown>[])"
              empty-message="Aucune action"
            >
              <template #cell-action_type="{ value }">
                <AppBadge :label="String(value)" :variant="actionVariant(String(value))" />
              </template>
            </DataTable>
          </section>

          <!-- Evenements de securite -->
          <section v-if="dossier.security_events.length > 0" class="dossier-section">
            <h3>Evenements de securite ({{ dossier.security_events.length }})</h3>
            <div class="security-events">
              <div v-for="evt in dossier.security_events" :key="evt.id" class="security-event-item">
                <AppBadge :label="evt.severity" :variant="severityVariant(evt.severity)" />
                <span class="event-type">{{ evt.event_type.replace("_", " ") }}</span>
                <span class="event-desc">{{ evt.description }}</span>
                <span class="mono event-date">{{ fmt(evt.created_at) }}</span>
              </div>
            </div>
          </section>

          <!-- Historique conduite -->
          <section v-if="dossier.conduct_log.length > 0" class="dossier-section">
            <h3>Historique de conduite ({{ dossier.conduct_log.length }})</h3>
            <DataTable
              :columns="dossierConductColumns"
              :rows="(dossier.conduct_log as unknown as Record<string, unknown>[])"
              empty-message="Aucun historique"
            >
              <template #cell-delta="{ value }">
                <span :class="['delta', Number(value) < 0 ? 'delta-neg' : 'delta-pos']">
                  {{ Number(value) > 0 ? '+' : '' }}{{ value }}
                </span>
              </template>
              <template #cell-created_at="{ value }">
                <span class="mono">{{ fmt(String(value)) }}</span>
              </template>
            </DataTable>
          </section>
        </template>
      </div>

      <!-- Placeholder quand aucun user selectionne -->
      <div v-else class="dossier-placeholder">
        <div class="placeholder-content">
          <svg class="placeholder-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
            <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" />
            <circle cx="12" cy="12" r="3" />
          </svg>
          <p>Selectionnez un utilisateur pour consulter son dossier</p>
        </div>
      </div>
    </div>

    <!-- Modale ajout surveillance -->
    <teleport to="body">
      <div v-if="addModalVisible" class="modal-overlay" @click.self="closeAddModal">
        <div class="modal-content">
          <div class="modal-header">
            <h3>Surveiller un membre</h3>
            <button class="modal-close" @click="closeAddModal">&times;</button>
          </div>

          <div class="modal-body">
            <label class="modal-label">Rechercher un membre</label>
            <div class="autocomplete-wrapper">
              <input
                v-model="addSearch"
                type="text"
                placeholder="Tapez le nom d'un membre..."
                class="modal-input"
                @input="onAddSearchInput"
                @focus="onAddSearchInput"
                @blur="onAddSearchBlur"
                autocomplete="off"
              />
              <div v-if="showAddSuggestions" class="autocomplete-list">
                <div
                  v-for="member in addSuggestions"
                  :key="member.id"
                  class="autocomplete-item"
                  @mousedown="selectAddMember(member)"
                >
                  <img v-if="member.avatar_url" :src="member.avatar_url" class="autocomplete-avatar" />
                  <div v-else class="autocomplete-avatar-placeholder">
                    {{ (member.display_name || member.username).charAt(0).toUpperCase() }}
                  </div>
                  <div class="autocomplete-info">
                    <span class="autocomplete-name">{{ member.display_name || member.username }}</span>
                    <span class="autocomplete-id">{{ member.id }}</span>
                  </div>
                </div>
              </div>
            </div>

            <div v-if="addSelectedMember" class="selected-member">
              <img v-if="addSelectedMember.avatar_url" :src="addSelectedMember.avatar_url" class="selected-avatar" />
              <div v-else class="autocomplete-avatar-placeholder">
                {{ (addSelectedMember.display_name || addSelectedMember.username).charAt(0).toUpperCase() }}
              </div>
              <div>
                <strong>{{ addSelectedMember.display_name || addSelectedMember.username }}</strong>
                <div class="autocomplete-id">{{ addSelectedMember.id }}</div>
              </div>
            </div>

            <label class="modal-label" style="margin-top: 16px;">Raison de la surveillance</label>
            <textarea
              v-model="addReason"
              class="modal-textarea"
              rows="2"
              placeholder="Pourquoi surveiller ce membre ? (optionnel)"
            ></textarea>
          </div>

          <div class="modal-footer">
            <button class="modal-cancel" @click="closeAddModal">Annuler</button>
            <button
              class="add-confirm-btn"
              :disabled="!addSelectedMember || addLoading"
              @click="confirmAddWatch"
            >
              {{ addLoading ? 'Ajout...' : 'Mettre en surveillance' }}
            </button>
          </div>
        </div>
      </div>
    </teleport>
  </div>
</template>

<style scoped>
.watched-users h1 {
  margin-bottom: 20px;
}

/* Filtres */
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

.risk-select {
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  min-width: 160px;
}

.risk-select:focus {
  outline: none;
  border-color: var(--accent);
}

/* Layout principal */
.content-layout {
  display: flex;
  gap: 20px;
  min-height: 0;
}

.users-list {
  width: 520px;
  min-width: 520px;
  display: flex;
  flex-direction: column;
  gap: 10px;
  overflow-y: auto;
  max-height: calc(100vh - 200px);
  padding-right: 4px;
}

/* Carte utilisateur */
.user-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
  cursor: pointer;
  transition: all 0.15s;
}

.user-card:hover {
  border-color: var(--accent);
  background-color: var(--bg-hover);
}

.user-card.selected {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.25);
}

.user-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
}

.user-identity {
  display: flex;
  align-items: center;
  gap: 10px;
}

.user-avatar-placeholder {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--accent), #7c5cfc);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 14px;
  color: white;
  flex-shrink: 0;
}

.user-names {
  display: flex;
  flex-direction: column;
  gap: 1px;
}

.user-name {
  font-weight: 600;
  font-size: 14px;
  color: var(--text-primary);
}

.user-id {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

/* Stats dans la carte */
.user-card-stats {
  display: flex;
  gap: 16px;
  margin-bottom: 10px;
}

.stat-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
}

.stat-value {
  font-weight: 700;
  font-size: 16px;
  color: var(--text-primary);
}

.stat-warn { color: var(--info); }
.stat-mute { color: var(--warning); }
.stat-ban { color: var(--danger); }

.stat-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.user-card-footer {
  display: flex;
  flex-wrap: wrap;
  gap: 8px;
  font-size: 11px;
  color: var(--text-secondary);
}

.last-incident .mono,
.mono {
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.security-count {
  color: var(--warning);
  font-weight: 600;
}

.conduct-info {
  color: var(--info);
}

/* Panneau dossier */
.dossier-panel {
  flex: 1;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 24px;
  overflow-y: auto;
  max-height: calc(100vh - 200px);
}

.dossier-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 20px;
}

.dossier-title {
  display: flex;
  align-items: center;
  gap: 12px;
}

.dossier-title h2 {
  font-size: 18px;
  margin: 0;
}

.close-btn {
  width: 32px;
  height: 32px;
  background: none;
  border-radius: 8px;
  font-size: 20px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

.close-btn:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

/* Resume du dossier */
.dossier-summary {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  margin-bottom: 24px;
}

.summary-card {
  background-color: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 120px;
}

.summary-value {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-primary);
  word-break: break-all;
}

.summary-value.conduct-low {
  color: var(--danger);
}

.summary-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

/* Sections du dossier */
.dossier-section {
  margin-bottom: 24px;
}

.dossier-section h3 {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 12px;
  color: var(--text-primary);
}

/* Evenements securite inline */
.security-events {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.security-event-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 10px 14px;
  background-color: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 13px;
}

.event-type {
  font-weight: 600;
  text-transform: capitalize;
}

.event-desc {
  flex: 1;
  color: var(--text-secondary);
}

.event-date {
  font-size: 11px;
  color: var(--text-secondary);
}

/* Delta conduite */
.delta {
  font-weight: 700;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.delta-neg { color: var(--danger); }
.delta-pos { color: var(--success); }

/* Placeholder */
.dossier-placeholder {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  min-height: 400px;
}

.placeholder-content {
  text-align: center;
  color: var(--text-secondary);
}

.placeholder-icon {
  width: 48px;
  height: 48px;
  margin-bottom: 12px;
  opacity: 0.4;
}

.placeholder-content p {
  font-size: 14px;
}

.loading, .empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}

.page-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.page-header-row h1 {
  margin: 0;
}

.add-watch-btn {
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 8px;
  padding: 10px 20px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.2s;
}

.add-watch-btn:hover {
  opacity: 0.85;
}

/* Modale */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  width: 100%;
  max-width: 480px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
}

.modal-header h3 { margin: 0; font-size: 16px; }
.modal-close { background: none; border: none; color: var(--text-secondary); font-size: 24px; cursor: pointer; }
.modal-close:hover { color: var(--text-primary); }

.modal-body { padding: 20px; }
.modal-label { display: block; font-size: 13px; font-weight: 600; color: var(--text-secondary); margin-bottom: 8px; }

.modal-input {
  width: 100%;
  background: var(--bg-input, var(--bg-card));
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  color: var(--text-primary);
  font-size: 14px;
  outline: none;
}

.modal-input:focus { border-color: var(--accent); }

.modal-textarea {
  width: 100%;
  background: var(--bg-input, var(--bg-card));
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  color: var(--text-primary);
  font-size: 14px;
  font-family: inherit;
  resize: vertical;
  outline: none;
}

.modal-textarea:focus { border-color: var(--accent); }

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 20px;
  border-top: 1px solid var(--border);
}

.modal-cancel {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 16px;
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
}

.modal-cancel:hover { background: var(--bg-hover); }

.add-confirm-btn {
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 6px;
  padding: 8px 20px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}

.add-confirm-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.selected-member {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
  padding: 12px;
  background: var(--bg-hover);
  border-radius: 8px;
}

.selected-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
}

/* Autocomplete */
.autocomplete-wrapper { position: relative; }

.autocomplete-list {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  margin-top: 4px;
  max-height: 200px;
  overflow-y: auto;
  z-index: 1001;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.autocomplete-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  cursor: pointer;
}

.autocomplete-item:hover { background: var(--bg-hover); }
.autocomplete-avatar { width: 28px; height: 28px; border-radius: 50%; flex-shrink: 0; }

.autocomplete-avatar-placeholder {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--accent), #6366f1);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  color: white;
  flex-shrink: 0;
}

.autocomplete-info { display: flex; flex-direction: column; gap: 1px; }
.autocomplete-name { font-size: 13px; font-weight: 600; }
.autocomplete-id { font-size: 11px; color: var(--text-secondary); font-family: monospace; }
</style>
