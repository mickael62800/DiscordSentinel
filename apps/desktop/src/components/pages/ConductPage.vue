<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useConduct, useConductDetail } from "../../composables/useConduct";
import { usePagination } from "../../composables/usePagination";
import { useGuildSelector } from "../../composables/useGuildSelector";
import AppBadge from "../atoms/AppBadge.vue";
import ErrorState from "../atoms/ErrorState.vue";
import PaginationBar from "../molecules/PaginationBar.vue";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();

const { selectedGuildId } = useGuildSelector();
const { config, leaderboard, loading, error, fetchLeaderboard } = useConduct();
const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedLeaderboard } = usePagination(leaderboard);
const { points: detailPoints, log: detailLog, loading: detailLoading, fetchDetail } = useConductDetail();

const selectedUser = ref<string | null>(null);
const adjustAmount = ref(1);
const adjustReason = ref("");
const adjusting = ref(false);

async function adjustPoints(positive: boolean) {
  if (!selectedGuildId.value || !selectedUser.value || !adjustReason.value) return;
  adjusting.value = true;
  try {
    const amount = positive ? Math.abs(adjustAmount.value) : -Math.abs(adjustAmount.value);
    await invoke("adjust_conduct_points", {
      guildId: selectedGuildId.value,
      userId: selectedUser.value,
      amount,
      reason: adjustReason.value,
    });
    adjustReason.value = "";
    await fetchDetail(selectedGuildId.value, selectedUser.value);
    await fetchLeaderboard();
  } catch (e) {
    console.error("Erreur ajustement points:", e);
  } finally {
    adjusting.value = false;
  }
}

function pointsVariant(points: number, max: number): "success" | "warning" | "danger" | "default" {
  const ratio = points / max;
  if (ratio > 0.66) return "success";
  if (ratio > 0.33) return "warning";
  if (points > 0) return "danger";
  return "default";
}

function pointsColor(points: number, max: number): string {
  const ratio = points / max;
  if (ratio > 0.66) return "#2ecc71";
  if (ratio > 0.33) return "#f39c12";
  if (points > 0) return "#e74c3c";
  return "#2c3e50";
}

async function selectUser(guildId: string, userId: string) {
  selectedUser.value = userId;
  await fetchDetail(guildId, userId);
}

function backToList() {
  selectedUser.value = null;
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>Points de conduite</h1>
      <p class="page-subtitle">Systeme de points type permis — 0 points = alerte moderateurs</p>
    </header>

    <!-- Config summary -->
    <div v-if="config" class="config-bar">
      <div class="config-item"><strong>Max:</strong> {{ config.max_points }} pts</div>
      <div class="config-item"><strong>Regen:</strong> +{{ config.regen_amount }}/{{ config.regen_interval === 'weekly' ? 'semaine' : 'mois' }}</div>
      <div class="config-item"><strong>Warn:</strong> -{{ config.penalty_warn }}</div>
      <div class="config-item"><strong>Delete:</strong> -{{ config.penalty_delete }}</div>
      <div class="config-item"><strong>Mute:</strong> -{{ config.penalty_mute }}</div>
      <div class="config-item"><strong>Ban:</strong> -{{ config.penalty_ban }}</div>
    </div>

    <!-- Pas de serveur selectionne -->
    <div v-if="!selectedGuildId" class="empty">
      Selectionnez un serveur dans la barre laterale pour voir les points de conduite.
    </div>

    <!-- Detail view -->
    <div v-else-if="selectedUser && detailPoints" class="detail-view">
      <button class="back-btn" @click="backToList">&larr; Retour</button>

      <div v-if="detailLoading" class="loading">Chargement...</div>
      <div v-else class="detail-content">
        <h2>{{ detailPoints.username }}</h2>
        <p class="user-id">ID : {{ detailPoints.user_id }}</p>
        <div class="points-display">
          <span class="points-value" :style="{ color: pointsColor(detailPoints.points, config?.max_points ?? 12) }">
            {{ detailPoints.points }}
          </span>
          <span class="points-max">/ {{ config?.max_points ?? 12 }}</span>
        </div>

        <!-- Ajuster les points -->
        <div class="adjust-section">
          <h3>Ajuster les points</h3>
          <div class="adjust-form">
            <input
              v-model.number="adjustAmount"
              type="number"
              min="1"
              max="12"
              class="adjust-input"
            />
            <input
              v-model="adjustReason"
              type="text"
              class="adjust-reason"
              placeholder="Raison de l'ajustement..."
            />
            <button
              class="adjust-btn add"
              :disabled="adjusting || !adjustReason"
              @click="adjustPoints(true)"
            >+ Ajouter</button>
            <button
              class="adjust-btn remove"
              :disabled="adjusting || !adjustReason"
              @click="adjustPoints(false)"
            >- Retirer</button>
          </div>
        </div>

        <h3>Historique des mouvements</h3>
        <div v-if="detailLog.length === 0" class="empty">Aucun mouvement</div>
        <table v-else class="data-table">
          <thead>
            <tr>
              <th>Date</th>
              <th>Delta</th>
              <th>Raison</th>
              <th>Avant</th>
              <th>Apres</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="entry in detailLog" :key="entry.id">
              <td>{{ fmt(entry.created_at) }}</td>
              <td :class="entry.delta < 0 ? 'text-danger' : 'text-success'">
                {{ entry.delta > 0 ? '+' : '' }}{{ entry.delta }}
              </td>
              <td>{{ entry.reason }}</td>
              <td>{{ entry.points_before }}</td>
              <td>{{ entry.points_after }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>

    <!-- Leaderboard -->
    <div v-else>
      <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchLeaderboard" />
      <div v-else-if="loading" class="loading">Chargement...</div>
      <div v-else-if="leaderboard.length === 0" class="empty">Aucun utilisateur avec des points</div>
      <table v-else class="data-table">
        <thead>
          <tr>
            <th>Utilisateur</th>
            <th>ID</th>
            <th>Points</th>
            <th>Statut</th>
            <th>Derniere regen</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="user in paginatedLeaderboard"
            :key="user.id"
            class="clickable"
            @click="selectUser(user.guild_id, user.user_id)"
          >
            <td>{{ user.username }}</td>
            <td class="mono">{{ user.user_id }}</td>
            <td>
              <span :style="{ color: pointsColor(user.points, config?.max_points ?? 12), fontWeight: 700 }">
                {{ user.points }}
              </span>
              / {{ config?.max_points ?? 12 }}
            </td>
            <td>
              <AppBadge :label="user.points === 0 ? 'ALERTE' : user.points <= 4 ? 'Critique' : user.points <= 8 ? 'Attention' : 'OK'" :variant="pointsVariant(user.points, config?.max_points ?? 12)" />
            </td>
            <td>{{ new Date(user.last_regen_at).toLocaleDateString() }}</td>
          </tr>
        </tbody>
      </table>

      <PaginationBar
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

<style scoped>
.page {
  padding: 24px;
}

.page-header {
  margin-bottom: 24px;
}

.page-header h1 {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-primary);
}

.page-subtitle {
  color: var(--text-secondary);
  font-size: 14px;
  margin-top: 4px;
}

.config-bar {
  display: flex;
  gap: 16px;
  padding: 12px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  margin-bottom: 24px;
  flex-wrap: wrap;
}

.config-item {
  font-size: 13px;
  color: var(--text-secondary);
}

.config-item strong {
  color: var(--text-primary);
}

.data-table {
  width: 100%;
  border-collapse: collapse;
}

.data-table th,
.data-table td {
  padding: 10px 14px;
  text-align: left;
  border-bottom: 1px solid var(--border);
  font-size: 13px;
}

.data-table th {
  color: var(--text-secondary);
  font-weight: 600;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.clickable {
  cursor: pointer;
}

.clickable:hover {
  background: var(--bg-hover);
}

.loading,
.empty {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary);
}

.back-btn {
  background: none;
  color: var(--accent);
  font-size: 14px;
  margin-bottom: 16px;
  cursor: pointer;
}

.back-btn:hover {
  text-decoration: underline;
}

.detail-content h2 {
  font-size: 20px;
  margin-bottom: 4px;
}

.user-id {
  font-size: 12px;
  color: var(--text-secondary);
  font-family: monospace;
  margin-bottom: 8px;
}

.mono {
  font-family: monospace;
  font-size: 12px;
  color: var(--text-secondary);
}

.detail-content h3 {
  font-size: 16px;
  margin-top: 24px;
  margin-bottom: 12px;
}

.points-display {
  margin-bottom: 24px;
}

.points-value {
  font-size: 48px;
  font-weight: 800;
}

.points-max {
  font-size: 24px;
  color: var(--text-secondary);
  margin-left: 4px;
}

.adjust-section {
  margin-bottom: 24px;
}

.adjust-section h3 {
  font-size: 16px;
  margin-bottom: 12px;
}

.adjust-form {
  display: flex;
  gap: 8px;
  align-items: center;
}

.adjust-input {
  width: 60px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 14px;
  text-align: center;
}

.adjust-reason {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
}

.adjust-reason::placeholder {
  color: var(--text-secondary);
}

.adjust-reason:focus,
.adjust-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(88, 101, 242, 0.2);
}

.adjust-btn {
  padding: 8px 16px;
  border: none;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.15s;
  white-space: nowrap;
}

.adjust-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.adjust-btn.add {
  background: var(--success-bg);
  color: var(--success);
  border: 1px solid var(--success);
}

.adjust-btn.add:hover:not(:disabled) {
  background: var(--success);
  color: white;
}

.adjust-btn.remove {
  background: var(--danger-bg);
  color: var(--danger);
  border: 1px solid var(--danger);
}

.adjust-btn.remove:hover:not(:disabled) {
  background: var(--danger);
  color: white;
}

.text-danger {
  color: var(--danger);
  font-weight: 600;
}

.text-success {
  color: #2ecc71;
  font-weight: 600;
}
</style>
