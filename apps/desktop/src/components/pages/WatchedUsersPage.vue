<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useWatchedUsers } from "../../composables/useWatchedUsers";
import { usePagination } from "../../composables/usePagination";
import { useGuildSelector } from "../../composables/useGuildSelector";
import AppBadge from "../atoms/AppBadge.vue";
import ErrorState from "../atoms/ErrorState.vue";
import PaginationBar from "../molecules/PaginationBar.vue";
import AddWatchModal from "../molecules/AddWatchModal.vue";
import UserDossierPanel from "../molecules/UserDossierPanel.vue";
import { getApiBaseUrl } from "../../utils/api";

import type { WatchedUser, UserActivity } from "../../types";
import { severityVariant } from "../../utils/variants";

// Tabs
const activeTab = ref<"all" | "manual" | "infractions">("all");

// Timeline activite
const activities = ref<UserActivity[]>([]);
const activitiesLoading = ref(false);

async function loadActivities(guildId: string, userId: string) {
  activitiesLoading.value = true;
  try {
    const baseUrl = await getApiBaseUrl();
    const resp = await fetch(`${baseUrl}/api/user-activity/${guildId}/${userId}?limit=50`);
    if (resp.ok) {
      activities.value = await resp.json();
    }
  } catch (e) {
    console.error("Erreur chargement activite:", e);
  } finally {
    activitiesLoading.value = false;
  }
}

const { selectedGuildId } = useGuildSelector();

const {
  users,
  loading,
  error: watchedError,
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
const addModalRef = ref<InstanceType<typeof AddWatchModal> | null>(null);

function openAddModal() {
  addModalVisible.value = true;
  // openReset apres le prochain tick pour que le ref soit monte
  setTimeout(() => addModalRef.value?.openReset(), 50);
}

function closeAddModal() {
  addModalVisible.value = false;
}

async function onAdded() {
  if (fetchUsers) await fetchUsers();
}

// Charger les activites quand un utilisateur est selectionne
watch(() => selectedUser.value, (user) => {
  if (user) {
    loadActivities(user.guild_id, user.user_id);
  } else {
    activities.value = [];
  }
});

const filteredUsers = computed(() => {
  let list = users.value;

  // Filtrer par tab
  if (activeTab.value === "manual") {
    list = list.filter((u) => u.total_warns === 0 && u.total_mutes === 0 && u.total_bans === 0);
  } else if (activeTab.value === "infractions") {
    list = list.filter((u) => u.total_warns > 0 || u.total_mutes > 0 || u.total_bans > 0);
  }

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
</script>

<template>
  <div class="watched-users">
    <div class="page-header-row">
      <h1>Surveillance des utilisateurs</h1>
      <button class="add-watch-btn" @click="openAddModal">+ Surveiller un membre</button>
    </div>

    <!-- Tabs -->
    <div class="tabs">
      <button :class="['tab', { active: activeTab === 'all' }]" @click="activeTab = 'all'">Tous</button>
      <button :class="['tab', { active: activeTab === 'manual' }]" @click="activeTab = 'manual'">Surveillance manuelle</button>
      <button :class="['tab', { active: activeTab === 'infractions' }]" @click="activeTab = 'infractions'">Infractions</button>
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

    <ErrorState v-if="watchedError" :message="watchedError" :retryable="true" @retry="fetchUsers" />
    <div v-else-if="loading" class="loading">Chargement...</div>

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
      <UserDossierPanel
        :user="selectedUser"
        :dossier="dossier"
        :dossier-loading="dossierLoading"
        :activities="activities"
        :activities-loading="activitiesLoading"
        @close="selectUser(null)"
        @removed="fetchUsers"
      />
    </div>

    <!-- Modale ajout surveillance -->
    <AddWatchModal
      ref="addModalRef"
      :visible="addModalVisible"
      :guild-id="selectedGuildId ?? ''"
      @close="closeAddModal"
      @added="onAdded"
    />
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
  width: 720px;
  min-width: 720px;
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

.loading, .empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}

/* Tabs */
.tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 4px;
}

.tab {
  flex: 1;
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}

.tab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.tab.active {
  background: var(--accent);
  color: white;
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
</style>
