<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import SettingsSection from "../organisms/SettingsSection.vue";
import AppInput from "../atoms/AppInput.vue";
import AppToggle from "../atoms/AppToggle.vue";
import AppButton from "../atoms/AppButton.vue";
import { useAuth } from "../../composables/useAuth";
import type { ApiConfig } from "../../types";

const router = useRouter();
const { clearConfig, logout } = useAuth();

const apiUrl = ref("http://localhost:3000");
const apiKey = ref("");
const autoRefresh = ref(true);
const refreshInterval = ref(5);
const apiSaving = ref(false);
const apiSaved = ref(false);
let savedTimer: ReturnType<typeof setTimeout> | null = null;

onMounted(async () => {
  try {
    const config = await invoke<ApiConfig | null>("get_api_config");
    if (config) {
      apiUrl.value = config.api_url;
      apiKey.value = config.api_key;
    }
  } catch (e) {
    console.error("Failed to load API config:", e);
  }
});

onUnmounted(() => {
  if (savedTimer) clearTimeout(savedTimer);
});

async function saveApiConfig() {
  apiSaving.value = true;
  apiSaved.value = false;
  try {
    await invoke("save_api_config", {
      apiUrl: apiUrl.value.trim(),
      apiKey: apiKey.value.trim(),
    });
    apiSaved.value = true;
    if (savedTimer) clearTimeout(savedTimer);
    savedTimer = setTimeout(() => { apiSaved.value = false; }, 2000);
  } catch (e) {
    console.error("Failed to save API config:", e);
  } finally {
    apiSaving.value = false;
  }
}

async function resetDiscordConfig() {
  try {
    await logout();
    await clearConfig();
    router.push("/setup");
  } catch (e) {
    console.error("Failed to reset config:", e);
  }
}
</script>

<template>
  <div class="settings">
    <h1>Settings</h1>

    <SettingsSection title="Backend API">
      <div class="setting-row">
        <label>API URL</label>
        <AppInput v-model="apiUrl" placeholder="http://localhost:3000" />
      </div>
      <div class="setting-row">
        <label>API Key (Bearer token)</label>
        <AppInput v-model="apiKey" type="password" placeholder="Leave empty if disabled" />
      </div>
      <div class="setting-row">
        <span class="setting-hint">Changes require an app restart to take effect.</span>
        <AppButton variant="primary" :disabled="apiSaving" @click="saveApiConfig">
          {{ apiSaved ? "Saved!" : apiSaving ? "Saving..." : "Save" }}
        </AppButton>
      </div>
    </SettingsSection>

    <SettingsSection title="Dashboard">
      <div class="setting-row">
        <label>Auto-refresh</label>
        <AppToggle v-model="autoRefresh" />
      </div>
      <div v-if="autoRefresh" class="setting-row">
        <label>Refresh interval (seconds)</label>
        <AppInput v-model="refreshInterval" type="number" :min="1" :max="60" />
      </div>
    </SettingsSection>

    <SettingsSection title="Discord Configuration">
      <div class="setting-row">
        <div>
          <label>Reset all credentials</label>
          <p class="setting-hint">Clears Discord OAuth + API config from LMDB and logs you out.</p>
        </div>
        <AppButton variant="secondary" class="danger-btn" @click="resetDiscordConfig">
          Reset
        </AppButton>
      </div>
    </SettingsSection>

    <SettingsSection title="About">
      <p class="about-text">DiscordSentinel v0.1.0</p>
      <p class="about-text secondary">Distributed moderation platform for Discord</p>
    </SettingsSection>
  </div>
</template>

<style scoped>
.settings h1 {
  margin-bottom: 24px;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
}

.setting-row label {
  color: var(--text-secondary);
  font-size: 14px;
}

.setting-hint {
  font-size: 12px;
  color: var(--text-secondary);
  opacity: 0.7;
  margin-top: 2px;
}

.danger-btn {
  background-color: transparent;
  border: 1px solid var(--border);
  color: var(--text-secondary);
}

.danger-btn:hover {
  border-color: var(--danger);
  color: var(--danger);
}

.about-text {
  font-size: 14px;
}

.about-text.secondary {
  color: var(--text-secondary);
  font-size: 13px;
  margin-top: 4px;
}
</style>
