<script setup lang="ts">
import { ref, watch } from "vue";
import { botConfigService } from "@/services/botConfigService";
import { useLevels } from "../../composables/useLevels";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import ErrorState from "../atoms/ErrorState.vue";
import AppTabs from "../molecules/AppTabs.vue";
import LevelsLeaderboardTab from "../organisms/LevelsLeaderboardTab.vue";
import LevelsRewardsTab from "../organisms/LevelsRewardsTab.vue";

const { config, rewards, loading, error, fetchAll } = useLevels();
const { selectedGuildId } = useGuildSelector();
useRealtimeRefresh(["xp_gained", "xp_admin_set", "xp_admin_reset"], fetchAll);

type PageTab = "leaderboard" | "rewards";
const pageTab = ref<PageTab>("leaderboard");

const pageTabs = [
  { key: "leaderboard", label: "Classement" },
  { key: "rewards", label: "Roles par niveau" },
];

// Mode de calcul XP pour les roles
const xpRoleMode = ref<string>("separate");
const xpRoleModeLoading = ref(false);

async function loadXpRoleMode() {
  if (!selectedGuildId.value) return;
  try {
    const configs = await botConfigService.getGuildConfig(selectedGuildId.value);
    const found = configs.find((c) => c.config_key === "xp_role_mode");
    xpRoleMode.value = found?.config_value ?? "separate";
  } catch {
    xpRoleMode.value = "separate";
  }
}

async function saveXpRoleMode(mode: string) {
  if (!selectedGuildId.value) return;
  xpRoleModeLoading.value = true;
  try {
    await botConfigService.set(selectedGuildId.value, "progression", "xp_role_mode", mode);
    xpRoleMode.value = mode;
  } catch (e) {
    console.error("Erreur sauvegarde xp_role_mode:", e);
  } finally {
    xpRoleModeLoading.value = false;
  }
}

watch(selectedGuildId, loadXpRoleMode, { immediate: true });
</script>

<template>
  <div class="levels page--constrained">
    <h1 class="page-title">Niveaux &amp; XP</h1>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchAll" />
    <div v-else-if="loading" class="loading">Chargement...</div>

    <template v-else>
      <!-- Config resume -->
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
        <div v-if="rewards.length > 0" class="config-item">
          <span class="config-value">{{ rewards.length }}</span>
          <span class="config-label">Recompenses</span>
        </div>
      </div>

      <!-- Mode de calcul XP -->
      <div class="xp-mode-bar">
        <span class="xp-mode-label">Mode d'attribution des roles :</span>
        <select
          class="xp-mode-select"
          :value="xpRoleMode"
          :disabled="xpRoleModeLoading"
          @change="saveXpRoleMode(($event.target as HTMLSelectElement).value)"
        >
          <option value="separate">Separe (texte = niveau texte, vocal = niveau vocal)</option>
          <option value="max">Le plus grand (max entre texte et vocal)</option>
          <option value="total">Total (XP texte + vocal combines)</option>
        </select>
      </div>

      <AppTabs
        :model-value="pageTab"
        :tabs="pageTabs"
        variant="plain"
        class="page-tabs-wrap"
        @update:model-value="(k) => (pageTab = k as PageTab)"
      />

      <LevelsLeaderboardTab v-if="pageTab === 'leaderboard'" />
      <LevelsRewardsTab v-else-if="pageTab === 'rewards'" />
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

.config-value { font-weight: 700; font-size: 18px; color: var(--text-primary); }
.config-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.text-success { color: var(--success); }
.text-danger { color: var(--danger); }

/* XP mode */
.xp-mode-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  padding: 10px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
}
.xp-mode-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  white-space: nowrap;
}
.xp-mode-select {
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-primary);
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  flex: 1;
  max-width: 450px;
}

.page-tabs-wrap { margin-bottom: 16px; }

@media (max-width: 768px) {
  .config-bar { gap: 8px; }
  .config-item {
    min-width: 0;
    flex: 1 1 calc(50% - 4px);
    padding: 10px 12px;
  }
  .config-value { font-size: 15px; }
  .xp-mode-bar {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
  }
  .xp-mode-select {
    max-width: 100%;
    width: 100%;
  }
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
