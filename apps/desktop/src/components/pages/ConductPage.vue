<script setup lang="ts">
import { ref } from "vue";
import { useConduct, useConductDetail } from "../../composables/useConduct";
import { useGuildSelector } from "../../composables/useGuildSelector";
import AppBadge from "../atoms/AppBadge.vue";

const { selectedGuildId } = useGuildSelector();
const { config, leaderboard, loading, fetchLeaderboard } = useConduct();
const { points: detailPoints, log: detailLog, loading: detailLoading, fetchDetail } = useConductDetail();

const selectedUser = ref<string | null>(null);
const editing = ref(false);

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
        <div class="points-display">
          <span class="points-value" :style="{ color: pointsColor(detailPoints.points, config?.max_points ?? 12) }">
            {{ detailPoints.points }}
          </span>
          <span class="points-max">/ {{ config?.max_points ?? 12 }}</span>
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
              <td>{{ new Date(entry.created_at).toLocaleString() }}</td>
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
      <div v-if="loading" class="loading">Chargement...</div>
      <div v-else-if="leaderboard.length === 0" class="empty">Aucun utilisateur avec des points</div>
      <table v-else class="data-table">
        <thead>
          <tr>
            <th>Utilisateur</th>
            <th>Points</th>
            <th>Statut</th>
            <th>Derniere regen</th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="user in leaderboard"
            :key="user.id"
            class="clickable"
            @click="selectUser(user.guild_id, user.user_id)"
          >
            <td>{{ user.username }}</td>
            <td>
              <span :style="{ color: pointsColor(user.points, config?.max_points ?? 12), fontWeight: 700 }">
                {{ user.points }}
              </span>
              / {{ config?.max_points ?? 12 }}
            </td>
            <td>
              <AppBadge :variant="pointsVariant(user.points, config?.max_points ?? 12)">
                {{ user.points === 0 ? 'ALERTE' : user.points <= 4 ? 'Critique' : user.points <= 8 ? 'Attention' : 'OK' }}
              </AppBadge>
            </td>
            <td>{{ new Date(user.last_regen_at).toLocaleDateString() }}</td>
          </tr>
        </tbody>
      </table>
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
  margin-bottom: 8px;
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

.text-danger {
  color: var(--danger);
  font-weight: 600;
}

.text-success {
  color: #2ecc71;
  font-weight: 600;
}
</style>
