<script setup lang="ts">
import { ref } from "vue";
import { useLevels } from "../../composables/useLevels";
import ErrorState from "../atoms/ErrorState.vue";
import type { UserLevel } from "../../types";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";

const { config, leaderboard, rewards, loading, error, fetchAll } = useLevels();
useRealtimeRefresh(["level_up", "xp_update"], fetchAll);

type ViewMode = "global" | "text" | "voice";
const viewMode = ref<ViewMode>("global");

function progressPercent(current: number, needed: number): number {
  if (needed <= 0) return 0;
  return Math.min(100, Math.round((current / needed) * 100));
}

function userLevel(user: UserLevel): number {
  if (viewMode.value === "text") return user.level_text;
  if (viewMode.value === "voice") return user.level_voice;
  return user.level;
}

function userXp(user: UserLevel): number {
  if (viewMode.value === "text") return user.xp_text;
  if (viewMode.value === "voice") return user.xp_voice;
  return user.xp;
}

function userCurrent(user: UserLevel): number {
  if (viewMode.value === "text") return user.xp_text_current;
  if (viewMode.value === "voice") return user.xp_voice_current;
  return user.xp_current;
}

function userNeeded(user: UserLevel): number {
  if (viewMode.value === "text") return user.xp_text_needed;
  if (viewMode.value === "voice") return user.xp_voice_needed;
  return user.xp_needed;
}

function sortedLeaderboard(): UserLevel[] {
  return [...leaderboard.value].sort((a, b) => userXp(b) - userXp(a));
}

function rewardForLevel(level: number, source: string): string | null {
  const r = rewards.value.find((rw) => rw.level === level && rw.source === source);
  return r ? r.role_id : null;
}

function hasReward(user: UserLevel): boolean {
  if (viewMode.value === "text") return !!rewardForLevel(user.level_text, "text");
  if (viewMode.value === "voice") return !!rewardForLevel(user.level_voice, "voice");
  return !!rewardForLevel(user.level_text, "text") || !!rewardForLevel(user.level_voice, "voice");
}
</script>

<template>
  <div class="levels">
    <h1>Niveaux & XP</h1>

    <div v-if="!config && !loading" class="empty">
      Selectionnez un serveur et configurez le systeme de niveaux.
    </div>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchAll" />
    <div v-else-if="loading" class="loading">Chargement...</div>

    <template v-else-if="config">
      <!-- Config resume -->
      <div class="config-bar">
        <div class="config-item">
          <span class="config-value">{{ config.xp_per_message }}</span>
          <span class="config-label">XP / message</span>
        </div>
        <div class="config-item">
          <span class="config-value">{{ config.xp_per_voice_minute }}</span>
          <span class="config-label">XP / min vocal</span>
        </div>
        <div class="config-item">
          <span class="config-value">{{ config.xp_cooldown_secs }}s</span>
          <span class="config-label">Cooldown</span>
        </div>
        <div class="config-item">
          <span :class="['config-value', config.enabled ? 'text-success' : 'text-danger']">
            {{ config.enabled ? "Actif" : "Inactif" }}
          </span>
          <span class="config-label">Statut</span>
        </div>
        <div v-if="rewards.length > 0" class="config-item">
          <span class="config-value">{{ rewards.length }}</span>
          <span class="config-label">Recompenses</span>
        </div>
      </div>

      <!-- View mode tabs -->
      <div class="view-tabs">
        <button :class="['tab', { active: viewMode === 'global' }]" @click="viewMode = 'global'">
          Global
        </button>
        <button :class="['tab tab-text', { active: viewMode === 'text' }]" @click="viewMode = 'text'">
          Texte
        </button>
        <button :class="['tab tab-voice', { active: viewMode === 'voice' }]" @click="viewMode = 'voice'">
          Vocal
        </button>
      </div>

      <!-- Leaderboard -->
      <div class="leaderboard">
        <div
          v-for="(user, index) in sortedLeaderboard()"
          :key="user.id"
          :class="['user-row', { 'top-3': index < 3 }]"
        >
          <div class="rank">
            <span :class="['rank-number', `rank-${index + 1}`]">{{ index + 1 }}</span>
          </div>
          <div class="user-avatar-placeholder">{{ user.username.charAt(0).toUpperCase() }}</div>
          <div class="user-info">
            <div class="user-header">
              <span class="user-name">{{ user.username }}</span>
              <span class="user-level">Niv. {{ userLevel(user) }}</span>
              <span v-if="hasReward(user)" class="reward-badge">Role</span>
            </div>
            <div class="progress-container">
              <div class="progress-bar">
                <div class="progress-fill" :style="{ width: progressPercent(userCurrent(user), userNeeded(user)) + '%' }"></div>
              </div>
              <span class="progress-text">{{ userCurrent(user) }} / {{ userNeeded(user) }} XP</span>
            </div>
            <!-- Mini stats texte/vocal en mode global -->
            <div v-if="viewMode === 'global'" class="mini-stats">
              <span class="mini-stat text">Texte Niv.{{ user.level_text }}</span>
              <span class="mini-stat voice">Vocal Niv.{{ user.level_voice }}</span>
            </div>
          </div>
          <div class="user-xp">
            <span class="xp-total">{{ userXp(user).toLocaleString() }}</span>
            <span class="xp-label">XP {{ viewMode === 'text' ? 'texte' : viewMode === 'voice' ? 'vocal' : 'total' }}</span>
          </div>
        </div>

        <div v-if="leaderboard.length === 0" class="empty">
          Aucun membre n'a encore d'XP sur ce serveur.
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.levels h1 {
  margin-bottom: 20px;
}

/* Config bar */
.config-bar {
  display: flex;
  gap: 16px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.config-item {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 20px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  min-width: 100px;
}

.config-value {
  font-weight: 700;
  font-size: 18px;
  color: var(--text-primary);
}

.config-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.text-success { color: var(--success); }
.text-danger { color: var(--danger); }

/* View tabs */
.view-tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.tab {
  padding: 8px 20px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.tab:hover {
  background: var(--bg-hover);
}

.tab.active {
  background: var(--accent);
  color: white;
  border-color: var(--accent);
}

.tab-text.active {
  background: #3498DB;
  border-color: #3498DB;
}

.tab-voice.active {
  background: #E91E63;
  border-color: #E91E63;
}

/* Leaderboard */
.leaderboard {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.user-row {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 14px 18px;
  display: flex;
  align-items: center;
  gap: 14px;
}

.user-row.top-3 {
  border-color: rgba(88, 101, 242, 0.3);
}

.rank {
  width: 32px;
  text-align: center;
}

.rank-number {
  font-weight: 700;
  font-size: 16px;
  color: var(--text-secondary);
}

.rank-1 { color: #FFD700; }
.rank-2 { color: #C0C0C0; }
.rank-3 { color: #CD7F32; }

.user-avatar-placeholder {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--accent), #7c5cfc);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 16px;
  color: white;
  flex-shrink: 0;
}

.user-info {
  flex: 1;
  min-width: 0;
}

.user-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.user-name {
  font-weight: 600;
  font-size: 14px;
  color: var(--text-primary);
}

.user-level {
  font-size: 12px;
  font-weight: 600;
  color: var(--accent);
  background-color: var(--accent-bg);
  padding: 2px 8px;
  border-radius: 4px;
}

.reward-badge {
  font-size: 10px;
  font-weight: 600;
  color: var(--warning);
  background-color: var(--warning-bg);
  padding: 2px 6px;
  border-radius: 4px;
}

.progress-container {
  display: flex;
  align-items: center;
  gap: 10px;
}

.progress-bar {
  flex: 1;
  height: 8px;
  background-color: var(--bg-hover);
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), #7c5cfc);
  border-radius: 4px;
  transition: width 0.3s;
}

.progress-text {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  white-space: nowrap;
  min-width: 100px;
  text-align: right;
}

/* Mini stats for global view */
.mini-stats {
  display: flex;
  gap: 8px;
  margin-top: 4px;
}

.mini-stat {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 3px;
}

.mini-stat.text {
  color: #3498DB;
  background: rgba(52, 152, 219, 0.1);
}

.mini-stat.voice {
  color: #E91E63;
  background: rgba(233, 30, 99, 0.1);
}

.user-xp {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  min-width: 80px;
}

.xp-total {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-primary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.xp-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
}

.loading, .empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}
</style>
