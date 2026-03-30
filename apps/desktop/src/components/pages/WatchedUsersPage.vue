<script setup lang="ts">
import { computed } from "vue";
import { useWatchedUsers } from "../../composables/useWatchedUsers";
import { usePagination } from "../../composables/usePagination";
import AppBadge from "../atoms/AppBadge.vue";
import DataTable from "../organisms/DataTable.vue";
import PaginationBar from "../molecules/PaginationBar.vue";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
import type { TableColumn, WatchedUser } from "../../types";
import { actionVariant, severityVariant } from "../../utils/variants";

const {
  users,
  loading,
  searchQuery,
  riskFilter,
  selectedUser,
  dossier,
  dossierLoading,
  selectUser,
} = useWatchedUsers();

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
    <h1>Surveillance des utilisateurs</h1>

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
      </div>

      <PaginationBar
        :current-page="currentPage"
        :total-pages="totalPages"
        :total-items="totalItems"
        :per-page="perPage"
        @update:current-page="currentPage = $event"
        @update:per-page="perPage = $event"
      />

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
  width: 420px;
  min-width: 420px;
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
</style>
