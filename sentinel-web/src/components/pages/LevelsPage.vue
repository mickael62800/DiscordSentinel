<script setup lang="ts">
import { useLevels } from "../../composables/useLevels";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import ErrorState from "../atoms/ErrorState.vue";
import LevelsLeaderboardTab from "../organisms/LevelsLeaderboardTab.vue";

const { config, loading, error, fetchAll } = useLevels();
useRealtimeRefresh(["xp_gained", "xp_admin_set", "xp_admin_reset"], fetchAll);
</script>

<template>
  <div class="levels page--constrained">
    <h1 class="page-title">Niveaux &amp; XP</h1>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchAll" />
    <div v-else-if="loading" class="loading">Chargement...</div>

    <template v-else>
      <div v-if="config" class="config-bar">
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
      </div>

      <LevelsLeaderboardTab />
    </template>
  </div>
</template>

<style scoped>
.levels h1 { margin-bottom: 20px; }

.loading {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}

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

.config-value { font-weight: 700; font-size: 18px; color: var(--text-primary); }
.config-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.text-success { color: var(--success); }
.text-danger { color: var(--danger); }

@media (max-width: 768px) {
  .config-bar { gap: 8px; }
  .config-item {
    min-width: 0;
    flex: 1 1 calc(50% - 4px);
    padding: 10px 12px;
  }
  .config-value { font-size: 15px; }
}

@media (max-width: 480px) {
  .config-bar {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  }
  .config-item {
    padding: 8px 10px;
    flex: initial;
  }
  .config-value { font-size: 14px; }
  .config-label { font-size: 9px; }
}
</style>
