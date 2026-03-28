<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { BotDefinition, BotGuildConfig, ConfigField } from "../../types";
import { useGuildSelector } from "../../composables/useGuildSelector";

const { selectedGuildId, selectedGuild, guilds } = useGuildSelector();

const workerNames = ["moderation-worker", "analytics-worker"];

const allDefinitions = ref<BotDefinition[]>([]);
const definitions = computed(() =>
  allDefinitions.value.filter((d) => !workerNames.includes(d.bot_name)),
);
const configs = ref<BotGuildConfig[]>([]);
const selectedBot = ref<string | null>(null);
const loading = ref(false);
const saving = ref(false);
const formValues = ref<Record<string, string>>({});
const successMessage = ref("");

const selectedDefinition = computed(() =>
  definitions.value.find((d) => d.bot_name === selectedBot.value) ?? null
);

const configFields = computed<ConfigField[]>(() => {
  if (!selectedDefinition.value) return [];
  const schema = selectedDefinition.value.config_schema;
  return Array.isArray(schema) ? schema : [];
});

async function fetchDefinitions() {
  try {
    allDefinitions.value = await invoke<BotDefinition[]>("get_bot_definitions");
  } catch (e) {
    console.error("Erreur chargement definitions bots:", e);
  }
}

async function fetchConfig() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  try {
    configs.value = await invoke<BotGuildConfig[]>("get_bot_guild_config", {
      guildId: selectedGuildId.value,
    });
    loadFormValues();
  } catch (e) {
    console.error("Erreur chargement config:", e);
  } finally {
    loading.value = false;
  }
}

function loadFormValues() {
  const values: Record<string, string> = {};
  if (selectedBot.value) {
    for (const cfg of configs.value.filter((c) => c.bot_name === selectedBot.value)) {
      values[cfg.config_key] = cfg.config_value;
    }
  }
  formValues.value = values;
}

async function saveConfig() {
  if (!selectedGuildId.value || !selectedBot.value) return;
  saving.value = true;
  successMessage.value = "";
  try {
    for (const field of configFields.value) {
      const value = formValues.value[field.key] ?? "";
      if (value) {
        await invoke("set_bot_config", {
          guildId: selectedGuildId.value,
          botName: selectedBot.value,
          configKey: field.key,
          configValue: value,
        });
      } else {
        await invoke("delete_bot_config", {
          guildId: selectedGuildId.value,
          botName: selectedBot.value,
          configKey: field.key,
        });
      }
    }
    successMessage.value = "Configuration enregistree";
    await fetchConfig();
    setTimeout(() => (successMessage.value = ""), 3000);
  } catch (e) {
    console.error("Erreur sauvegarde:", e);
  } finally {
    saving.value = false;
  }
}

function selectBot(botName: string) {
  selectedBot.value = botName;
  loadFormValues();
}

onMounted(() => {
  fetchDefinitions();
  if (selectedGuildId.value) fetchConfig();
});

watch(selectedGuildId, () => {
  if (selectedGuildId.value) fetchConfig();
});

watch(selectedBot, loadFormValues);
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>Configuration des bots</h1>
      <p class="page-subtitle">Parametrer chaque bot pour le serveur selectionne</p>
    </header>

    <!-- Pas de serveur selectionne -->
    <div v-if="!selectedGuildId" class="empty-state">
      <p>Selectionnez un serveur dans la barre laterale pour configurer les bots.</p>
    </div>

    <template v-else>
      <div class="server-info">
        <span class="server-label">Serveur :</span>
        <span class="server-name">{{ selectedGuild?.name }}</span>
      </div>

      <!-- Liste des bots -->
      <div class="bot-grid">
        <div
          v-for="bot in definitions"
          :key="bot.bot_name"
          class="bot-card"
          :class="{ active: selectedBot === bot.bot_name }"
          @click="selectBot(bot.bot_name)"
        >
          <div class="bot-name">{{ bot.display_name }}</div>
          <div class="bot-desc">{{ bot.description }}</div>
          <div class="bot-params">
            {{ bot.config_schema.length }} parametre{{ bot.config_schema.length > 1 ? "s" : "" }}
          </div>
        </div>
      </div>

      <!-- Formulaire de config -->
      <div v-if="selectedDefinition" class="config-form">
        <h2>{{ selectedDefinition.display_name }}</h2>

        <div v-if="configFields.length === 0" class="no-params">
          Ce bot n'a pas de parametres configurables par serveur.
        </div>

        <template v-else>
          <div v-for="field in configFields" :key="field.key" class="form-group">
            <label :for="field.key" class="form-label">
              {{ field.label }}
              <span v-if="field.required" class="required">*</span>
            </label>
            <input
              :id="field.key"
              v-model="formValues[field.key]"
              class="form-input"
              :placeholder="'ID du ' + field.type"
              type="text"
            />
            <span class="form-hint">Type : {{ field.type }} — Cle : {{ field.key }}</span>
          </div>

          <div class="form-actions">
            <button class="btn-save" :disabled="saving" @click="saveConfig">
              {{ saving ? "Enregistrement..." : "Enregistrer" }}
            </button>
            <span v-if="successMessage" class="success-msg">{{ successMessage }}</span>
          </div>
        </template>
      </div>
    </template>
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

.empty-state {
  text-align: center;
  padding: 60px 20px;
  color: var(--text-secondary);
  font-size: 15px;
}

.server-info {
  margin-bottom: 20px;
  padding: 10px 16px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 14px;
}

.server-label {
  color: var(--text-secondary);
  margin-right: 8px;
}

.server-name {
  font-weight: 600;
  color: var(--text-primary);
}

.bot-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 12px;
  margin-bottom: 24px;
}

.bot-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 16px;
  cursor: pointer;
  transition: border-color 0.15s;
}

.bot-card:hover {
  border-color: var(--accent);
}

.bot-card.active {
  border-color: var(--accent);
  background: rgba(99, 102, 241, 0.08);
}

.bot-name {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary);
  margin-bottom: 4px;
}

.bot-desc {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.bot-params {
  font-size: 11px;
  color: var(--accent);
  font-weight: 500;
}

.config-form {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 24px;
}

.config-form h2 {
  font-size: 18px;
  margin-bottom: 20px;
  color: var(--text-primary);
}

.no-params {
  color: var(--text-secondary);
  font-size: 14px;
  padding: 20px 0;
}

.form-group {
  margin-bottom: 16px;
}

.form-label {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 6px;
}

.required {
  color: var(--danger);
}

.form-input {
  width: 100%;
  padding: 10px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 14px;
  font-family: monospace;
}

.form-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}

.form-hint {
  display: block;
  font-size: 11px;
  color: var(--text-secondary);
  margin-top: 4px;
}

.form-actions {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 20px;
}

.btn-save {
  padding: 10px 24px;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 8px;
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
}

.btn-save:hover:not(:disabled) {
  opacity: 0.9;
}

.btn-save:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.success-msg {
  color: var(--success);
  font-size: 13px;
  font-weight: 500;
}
</style>
