<script setup lang="ts">
import { useLevels } from "../../composables/useLevels";
import type { UserLevel } from "../../types";

const { config, leaderboard, rewards, loading } = useLevels();

function progressPercent(user: UserLevel): number {
  if (user.xp_needed <= 0) return 0;
  return Math.min(100, Math.round((user.xp_current / user.xp_needed) * 100));
}

function rewardForLevel(level: number): string | null {
  const r = rewards.value.find((rw) => rw.level === level);
  return r ? r.role_id : null;
}
</script>

<template>
  <div class="levels">
    <h1>Niveaux & XP</h1>

    <div v-if="!config && !loading" class="empty">
      Selectionnez un serveur et configurez le systeme de niveaux.
    </div>

    <div v-if="loading" class="loading">Chargement...</div>

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

      <!-- Leaderboard -->
      <div class="leaderboard">
        <div
          v-for="(user, index) in leaderboard"
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
              <span class="user-level">Niv. {{ user.level }}</span>
              <span v-if="rewardForLevel(user.level)" class="reward-badge">Role</span>
            </div>
            <div class="progress-container">
              <div class="progress-bar">
                <div class="progress-fill" :style="{ width: progressPercent(user) + '%' }"></div>
              </div>
              <span class="progress-text">{{ user.xp_current }} / {{ user.xp_needed }} XP</span>
            </div>
          </div>
          <div class="user-xp">
            <span class="xp-total">{{ user.xp.toLocaleString() }}</span>
            <span class="xp-label">XP total</span>
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
  margin-bottom: 24px;
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
  background-color: rgba(88, 101, 242, 0.15);
  padding: 2px 8px;
  border-radius: 4px;
}

.reward-badge {
  font-size: 10px;
  font-weight: 600;
  color: var(--warning);
  background-color: rgba(254, 231, 92, 0.15);
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
