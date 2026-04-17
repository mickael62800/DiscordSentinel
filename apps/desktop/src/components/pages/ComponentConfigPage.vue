<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import type { BotDefinition, BotGuildConfig, ConfigField } from "../../types";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useToast } from "../../composables/useToast";
import AppBadge from "../atoms/AppBadge.vue";
import AppToggle from "../atoms/AppToggle.vue";
import BotTokenManager from "../molecules/BotTokenManager.vue";

const { success, error: showError } = useToast();

const { selectedGuildId, selectedGuild } = useGuildSelector();

const workerNames = ["moderation-worker", "analytics-worker", "cache-worker", "cleanup-worker", "coude-worker", "monitoring-worker"];

// Workers qui n'ont PAS besoin de token Discord
const workersWithoutToken = ["analytics-worker", "cache-worker", "cleanup-worker", "moderation-worker", "monitoring-worker"];

function needsToken(botName: string): boolean {
  return !workersWithoutToken.includes(botName);
}

const definitions = ref<BotDefinition[]>([]);
const configs = ref<BotGuildConfig[]>([]);
const selectedComponent = ref<string | null>(null);
const loading = ref(false);
const saving = ref(false);
const formValues = ref<Record<string, string>>({});
const savedValues = ref<Record<string, string>>({});
const successMessage = ref("");

// ── Token status (map utilisée pour le badge ; la gestion CRUD vit dans BotTokenManager) ──
const tokenMap = ref<Record<string, boolean>>({});

async function fetchTokens() {
  try {
    const tokens = await invoke<[string, boolean][]>("get_all_bot_tokens");
    const map: Record<string, boolean> = {};
    for (const [name, has] of tokens) {
      map[name] = has;
    }
    tokenMap.value = map;
  } catch (e) {
    console.error("Erreur chargement tokens:", e);
  }
}

function isWorker(botName: string): boolean {
  return workerNames.includes(botName);
}

const moduleDefinitions = computed(() =>
  definitions.value.filter((d) => !isWorker(d.bot_name)),
);

const workerDefinitions = computed(() =>
  definitions.value.filter((d) => isWorker(d.bot_name)),
);

const selectedDefinition = computed(() =>
  definitions.value.find((d) => d.bot_name === selectedComponent.value) ?? null,
);

const selectedIsWorker = computed(() =>
  selectedComponent.value ? isWorker(selectedComponent.value) : false,
);

const configFields = computed<ConfigField[]>(() => {
  if (!selectedDefinition.value) return [];
  const schema = selectedDefinition.value.config_schema;
  return Array.isArray(schema) ? schema : [];
});

const booleanFields = computed(() => configFields.value.filter((f) => f.type === "boolean"));
const numberFields = computed(() => configFields.value.filter((f) => f.type === "number"));
const channelFields = computed(() => configFields.value.filter((f) => f.type === "channel"));
const roleFields = computed(() => configFields.value.filter((f) => f.type === "role"));
const textFields = computed(() => configFields.value.filter((f) => f.type === "text"));

const allTogglesOn = computed(() =>
  booleanFields.value.length > 0 && booleanFields.value.every((f) => formValues.value[f.key] === "true" || formValues.value[f.key] === "1"),
);

function enableAllToggles() {
  for (const field of booleanFields.value) {
    formValues.value[field.key] = "true";
  }
}

function disableAllToggles() {
  for (const field of booleanFields.value) {
    formValues.value[field.key] = "false";
  }
}

function isFieldModified(key: string): boolean {
  return (formValues.value[key] ?? "") !== (savedValues.value[key] ?? "");
}

const hasChanges = computed(() =>
  configFields.value.some((f) => isFieldModified(f.key)),
);

const changesCount = computed(() =>
  configFields.value.filter((f) => isFieldModified(f.key)).length,
);

function fieldStatus(field: ConfigField): { text: string; source: "db" | "default" | "none" } {
  const dbValue = savedValues.value[field.key];

  if (selectedIsWorker.value) {
    // Worker: show units (heure(s) / minute(s))
    if (dbValue !== undefined && dbValue !== "") {
      const unit = field.label.includes("heure") ? "heure(s)" : "minute(s)";
      return { text: `Valeur actuelle : ${dbValue} ${unit}`, source: "db" };
    }
    if (field.default !== undefined && field.default !== "") {
      const unit = field.label.includes("heure") ? "heure(s)" : "minute(s)";
      return { text: `Valeur par defaut : ${field.default} ${unit}`, source: "default" };
    }
    return { text: "Non configure", source: "none" };
  }

  // Module: show type descriptions
  const typeLabel =
    field.type === "channel" ? "ID du salon"
    : field.type === "role" ? "ID du role"
    : field.type === "number" ? "nombre"
    : field.type === "boolean" ? "true/false"
    : "texte";

  if (dbValue !== undefined && dbValue !== "") {
    return { text: `Configure : ${dbValue}`, source: "db" };
  }
  if (field.default !== undefined && field.default !== "") {
    return { text: `Par defaut : ${field.default} (${typeLabel})`, source: "default" };
  }
  return { text: `Non configure (${typeLabel})`, source: "none" };
}

async function fetchDefinitions() {
  try {
    definitions.value = await invoke<BotDefinition[]>("get_bot_definitions");
  } catch (e) {
    console.error("Erreur chargement definitions:", e);
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
  if (selectedComponent.value) {
    for (const cfg of configs.value.filter((c) => c.bot_name === selectedComponent.value)) {
      values[cfg.config_key] = cfg.config_value;
    }
  }
  savedValues.value = { ...values };
  formValues.value = { ...values };
}

function cancelChanges() {
  formValues.value = { ...savedValues.value };
}

async function saveConfig() {
  if (!selectedGuildId.value || !selectedComponent.value) return;
  saving.value = true;
  successMessage.value = "";
  try {
    for (const field of configFields.value) {
      if (!isFieldModified(field.key)) continue;
      const value = formValues.value[field.key] ?? "";
      if (value) {
        await invoke("set_bot_config", {
          guildId: selectedGuildId.value,
          botName: selectedComponent.value,
          configKey: field.key,
          configValue: String(value),
        });
      } else {
        await invoke("delete_bot_config", {
          guildId: selectedGuildId.value,
          botName: selectedComponent.value,
          configKey: field.key,
        });
      }
    }
    successMessage.value = `${changesCount.value} parametre(s) enregistre(s)`;
    await fetchConfig();
    success(`${changesCount.value} parametre(s) enregistre(s)`);
    setTimeout(() => (successMessage.value = ""), 3000);
  } catch (e) {
    console.error("Erreur sauvegarde:", e);
    showError("Erreur lors de la sauvegarde de la configuration");
  } finally {
    saving.value = false;
  }
}

function selectComponent(name: string) {
  selectedComponent.value = name;
  loadFormValues();
}

onMounted(() => {
  fetchDefinitions();
  fetchTokens();
  if (selectedGuildId.value) fetchConfig();
});

watch(selectedGuildId, () => {
  if (selectedGuildId.value) fetchConfig();
});

watch(selectedComponent, loadFormValues);
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>Configuration des composants</h1>
      <p class="page-subtitle">Parametrer chaque module et worker pour le serveur selectionne</p>
    </header>

    <div v-if="!selectedGuildId" class="empty-state">
      <p>Selectionnez un serveur dans la barre laterale pour configurer les composants.</p>
    </div>

    <template v-else>
      <div class="server-info">
        <span class="server-label">Serveur :</span>
        <span class="server-name">{{ selectedGuild?.name }}</span>
      </div>

      <!-- Section Modules -->
      <section class="component-section">
        <div class="section-header">
          <h2 class="section-heading">Modules</h2>
          <span class="section-count">{{ moduleDefinitions.length }}</span>
        </div>
        <div class="component-grid">
          <div
            v-for="def in moduleDefinitions"
            :key="def.bot_name"
            class="component-card"
            :class="{ active: selectedComponent === def.bot_name }"
            @click="selectComponent(def.bot_name)"
          >
            <div class="component-card-header">
              <div class="component-name">{{ def.display_name }}</div>
              <div class="component-badges">
                <span
                  v-if="needsToken(def.bot_name)"
                  class="token-badge"
                  :class="tokenMap[def.bot_name] ? 'token-ok' : 'token-missing'"
                >
                  {{ tokenMap[def.bot_name] ? 'Token OK' : 'Pas de token' }}
                </span>
                <AppBadge label="Module" variant="info" />
              </div>
            </div>
            <div class="component-desc">{{ def.description }}</div>
            <div class="component-params">
              {{ def.config_schema.length }} parametre{{ def.config_schema.length > 1 ? "s" : "" }}
            </div>

            <BotTokenManager
              v-if="needsToken(def.bot_name)"
              :bot-name="def.bot_name"
              :has-token="!!tokenMap[def.bot_name]"
              @updated="fetchTokens"
            />
          </div>
        </div>
      </section>

      <!-- Section Workers -->
      <section class="component-section">
        <div class="section-header">
          <h2 class="section-heading">Workers</h2>
          <span class="section-count">{{ workerDefinitions.length }}</span>
        </div>
        <div class="component-grid">
          <div
            v-for="def in workerDefinitions"
            :key="def.bot_name"
            class="component-card"
            :class="{ active: selectedComponent === def.bot_name }"
            @click="selectComponent(def.bot_name)"
          >
            <div class="component-card-header">
              <div class="component-name">{{ def.display_name }}</div>
              <div class="component-badges">
                <span
                  v-if="needsToken(def.bot_name)"
                  class="token-badge"
                  :class="tokenMap[def.bot_name] ? 'token-ok' : 'token-missing'"
                >
                  {{ tokenMap[def.bot_name] ? 'Token OK' : 'Pas de token' }}
                </span>
                <AppBadge label="Worker" variant="warning" />
              </div>
            </div>
            <div class="component-desc">{{ def.description }}</div>
            <div class="component-params">
              {{ def.config_schema.length }} parametre{{ def.config_schema.length > 1 ? "s" : "" }}
            </div>

            <BotTokenManager
              v-if="needsToken(def.bot_name)"
              :bot-name="def.bot_name"
              :has-token="!!tokenMap[def.bot_name]"
              @updated="fetchTokens"
            />
          </div>
        </div>
      </section>

      <!-- Config form -->
      <div v-if="selectedDefinition" class="config-form">
        <div class="config-form-header">
          <h2>{{ selectedDefinition.display_name }}</h2>
          <AppBadge
            v-if="selectedIsWorker"
            label="Worker"
            variant="warning"
          />
          <AppBadge
            v-else
            label="Module"
            variant="info"
          />
        </div>

        <div v-if="configFields.length === 0" class="no-params">
          Ce composant n'a pas de parametres configurables par serveur.
        </div>

        <template v-else>
          <!-- Section toggles (6 par ligne) -->
          <div v-if="booleanFields.length > 0" class="toggles-section">
            <div class="section-title-row">
              <h3 class="section-title">Fonctionnalites</h3>
              <button
                class="btn-toggle-all"
                @click="allTogglesOn ? disableAllToggles() : enableAllToggles()"
              >
                {{ allTogglesOn ? 'Tout desactiver' : 'Tout activer' }}
              </button>
            </div>
            <div class="toggles-grid">
              <div
                v-for="field in booleanFields"
                :key="field.key"
                class="toggle-card"
                :class="{ modified: isFieldModified(field.key) }"
              >
                <div class="toggle-card-header">
                  <span class="toggle-card-label">{{ field.label }}</span>
                  <span v-if="field.description" class="tooltip-wrap">
                    <span class="info-icon">i</span>
                    <span class="tooltip-text">{{ field.description }}</span>
                  </span>
                  <span v-if="isFieldModified(field.key)" class="modified-dot"></span>
                </div>
                <div class="toggle-card-control">
                  <AppToggle
                    :model-value="formValues[field.key] === 'true' || formValues[field.key] === '1'"
                    @update:model-value="formValues[field.key] = $event ? 'true' : 'false'"
                  />
                  <span class="toggle-state" :class="{ active: formValues[field.key] === 'true' || formValues[field.key] === '1' }">
                    {{ formValues[field.key] === 'true' || formValues[field.key] === '1' ? 'ON' : 'OFF' }}
                  </span>
                </div>
              </div>
            </div>
          </div>

          <!-- Section Valeurs numeriques -->
          <div v-if="numberFields.length > 0" class="inputs-section">
            <h3 class="section-title">Valeurs</h3>
            <div class="fields-grid">
              <div
                v-for="field in numberFields"
                :key="field.key"
                class="field-card"
                :class="{ modified: isFieldModified(field.key) }"
              >
                <label :for="field.key" class="form-label">
                  {{ field.label }}
                  <span v-if="field.description" class="tooltip-wrap">
                    <span class="info-icon">i</span>
                    <span class="tooltip-text">{{ field.description }}</span>
                  </span>
                  <span v-if="isFieldModified(field.key)" class="modified-dot"></span>
                </label>
                <input
                  :id="field.key"
                  v-model="formValues[field.key]"
                  class="form-input form-input-number"
                  type="number"
                  min="0"
                  :placeholder="field.default !== undefined ? String(field.default) : '0'"
                />
                <span class="form-hint" :class="'hint-' + fieldStatus(field).source">{{ fieldStatus(field).text }}</span>
              </div>
            </div>
          </div>

          <!-- Section Salons -->
          <div v-if="channelFields.length > 0" class="inputs-section">
            <h3 class="section-title">Salons</h3>
            <div class="fields-grid">
              <div
                v-for="field in channelFields"
                :key="field.key"
                class="field-card"
                :class="{ modified: isFieldModified(field.key) }"
              >
                <label :for="field.key" class="form-label">
                  {{ field.label }}
                  <span v-if="field.description" class="tooltip-wrap">
                    <span class="info-icon">i</span>
                    <span class="tooltip-text">{{ field.description }}</span>
                  </span>
                  <span v-if="isFieldModified(field.key)" class="modified-dot"></span>
                </label>
                <input
                  :id="field.key"
                  v-model="formValues[field.key]"
                  class="form-input"
                  type="text"
                  placeholder="ID du salon Discord"
                />
                <span class="form-hint" :class="'hint-' + fieldStatus(field).source">{{ fieldStatus(field).text }}</span>
              </div>
            </div>
          </div>

          <!-- Section Roles -->
          <div v-if="roleFields.length > 0" class="inputs-section">
            <h3 class="section-title">Roles</h3>
            <div class="fields-grid">
              <div
                v-for="field in roleFields"
                :key="field.key"
                class="field-card"
                :class="{ modified: isFieldModified(field.key) }"
              >
                <label :for="field.key" class="form-label">
                  {{ field.label }}
                  <span v-if="field.description" class="tooltip-wrap">
                    <span class="info-icon">i</span>
                    <span class="tooltip-text">{{ field.description }}</span>
                  </span>
                  <span v-if="isFieldModified(field.key)" class="modified-dot"></span>
                </label>
                <input
                  :id="field.key"
                  v-model="formValues[field.key]"
                  class="form-input"
                  type="text"
                  placeholder="ID du role Discord"
                />
                <span class="form-hint" :class="'hint-' + fieldStatus(field).source">{{ fieldStatus(field).text }}</span>
              </div>
            </div>
          </div>

          <!-- Section Texte / Listes -->
          <div v-if="textFields.length > 0" class="inputs-section">
            <h3 class="section-title">Texte & Listes</h3>
            <div class="fields-grid fields-grid-text">
              <div
                v-for="field in textFields"
                :key="field.key"
                class="field-card field-card-wide"
                :class="{ modified: isFieldModified(field.key) }"
              >
                <label :for="field.key" class="form-label">
                  {{ field.label }}
                  <span v-if="field.description" class="tooltip-wrap">
                    <span class="info-icon">i</span>
                    <span class="tooltip-text">{{ field.description }}</span>
                  </span>
                  <span v-if="isFieldModified(field.key)" class="modified-dot"></span>
                </label>
                <input
                  :id="field.key"
                  v-model="formValues[field.key]"
                  class="form-input"
                  type="text"
                  :placeholder="field.default !== undefined ? String(field.default) : ''"
                />
                <span class="form-hint" :class="'hint-' + fieldStatus(field).source">{{ fieldStatus(field).text }}</span>
              </div>
            </div>
          </div>

          <div class="form-actions">
            <button
              class="btn-save"
              :disabled="saving || !hasChanges"
              @click="saveConfig"
            >
              {{ saving ? "Enregistrement..." : hasChanges ? `Enregistrer (${changesCount})` : "Aucune modification" }}
            </button>
            <button
              v-if="hasChanges"
              class="btn-cancel"
              @click="cancelChanges"
            >
              Annuler
            </button>
            <span v-if="successMessage" class="success-msg">{{ successMessage }}</span>
          </div>
        </template>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* Tooltip info icon */
.tooltip-wrap {
  position: relative;
  display: inline-flex;
  align-items: center;
  margin-left: 4px;
}

.info-icon {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 16px;
  height: 16px;
  border-radius: 50%;
  background: var(--accent);
  color: white;
  font-size: 10px;
  font-weight: 700;
  font-style: italic;
  cursor: help;
  flex-shrink: 0;
}

.tooltip-text {
  display: none;
  position: absolute;
  bottom: calc(100% + 8px);
  left: 50%;
  transform: translateX(-50%);
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-primary);
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 12px;
  font-weight: 400;
  font-style: normal;
  white-space: normal;
  width: max-content;
  max-width: 280px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.3);
  z-index: 100;
  line-height: 1.4;
}

.tooltip-text::after {
  content: "";
  position: absolute;
  top: 100%;
  left: 50%;
  transform: translateX(-50%);
  border: 6px solid transparent;
  border-top-color: var(--border);
}

.tooltip-wrap:hover .tooltip-text {
  display: block;
}

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

.component-section {
  margin-bottom: 24px;
}

.section-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border);
}

.section-heading {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0;
}

.section-count {
  font-size: 11px;
  font-weight: 600;
  color: var(--accent);
  background: rgba(99, 102, 241, 0.12);
  padding: 2px 8px;
  border-radius: 10px;
}

.component-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
}

.component-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 16px;
  cursor: pointer;
  transition: border-color 0.15s;
}

.component-card:hover {
  border-color: var(--accent);
}

.component-card.active {
  border-color: var(--accent);
  background: rgba(99, 102, 241, 0.08);
}

.component-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.component-badges {
  display: flex;
  align-items: center;
  gap: 6px;
}

.component-name {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary);
}

.component-desc {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.component-params {
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

.config-form-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 20px;
}

.config-form-header h2 {
  font-size: 18px;
  color: var(--text-primary);
}

.no-params {
  color: var(--text-secondary);
  font-size: 14px;
  padding: 20px 0;
}

/* Grid 3 colonnes pour les champs */
.fields-grid {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 16px;
  margin-top: 12px;
  margin-bottom: 8px;
}

.fields-grid-text {
  grid-template-columns: repeat(2, 1fr);
}

.field-card {
  padding: 16px 18px;
  border-radius: 10px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  transition: border-color 0.2s, background 0.2s;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.field-card.modified {
  border-color: var(--accent);
  background: rgba(99, 102, 241, 0.04);
}

.field-card-wide {
  grid-column: span 1;
}

.form-group {
  margin-bottom: 16px;
  padding: 12px;
  border-radius: 8px;
  border: 1px solid transparent;
  transition: border-color 0.2s, background 0.2s;
}

.form-group.modified {
  border-color: var(--accent);
  background: rgba(99, 102, 241, 0.04);
}

.form-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  margin-bottom: 4px;
  min-height: 20px;
}

.required {
  color: var(--danger);
}

.modified-badge {
  font-size: 10px;
  font-weight: 600;
  color: var(--accent);
  background: rgba(99, 102, 241, 0.12);
  padding: 1px 6px;
  border-radius: 4px;
  text-transform: uppercase;
}

.input-row {
  display: flex;
  align-items: center;
  gap: 8px;
}

.input-unit {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  min-width: 30px;
}

.form-input {
  width: 100%;
  padding: 11px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 14px;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  margin-top: auto;
}

/* Masquer les fleches up/down des inputs number */
.form-input-number {
  -moz-appearance: textfield;
  text-align: center;
  font-size: 16px;
  font-weight: 600;
  letter-spacing: 0.5px;
  padding: 12px 14px;
}

.form-input-number::-webkit-inner-spin-button,
.form-input-number::-webkit-outer-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

.input-row .form-input {
  flex: 1;
  max-width: 200px;
}

.form-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
}

.form-input::placeholder {
  color: var(--text-secondary);
  opacity: 0.5;
  font-style: italic;
}

.form-hint {
  display: block;
  font-size: 11px;
  margin-top: 4px;
}

.hint-db {
  color: #22c55e;
}

.hint-default {
  color: var(--text-secondary);
  font-style: italic;
}

.hint-none {
  color: var(--text-secondary);
  opacity: 0.6;
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
  opacity: 0.4;
  cursor: not-allowed;
}

.btn-cancel {
  padding: 10px 20px;
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  font-size: 14px;
  cursor: pointer;
}

.btn-cancel:hover {
  border-color: var(--danger);
  color: var(--danger);
}

.success-msg {
  color: var(--success);
  font-size: 13px;
  font-weight: 500;
}

.section-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 24px 0 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border);
}

.toggles-section:first-child .section-title-row {
  margin-top: 0;
}

.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0;
  padding: 0;
  border: none;
}

.inputs-section {
  margin-top: 24px;
  margin-bottom: 16px;
}

.inputs-section + .inputs-section {
  margin-top: 32px;
}

.btn-toggle-all {
  padding: 5px 14px;
  border: 1px solid var(--accent);
  border-radius: 6px;
  background: rgba(99, 102, 241, 0.08);
  color: var(--accent);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.btn-toggle-all:hover {
  background: var(--accent);
  color: white;
}

.toggles-grid {
  display: grid;
  grid-template-columns: repeat(6, 1fr);
  gap: 10px;
}

.toggle-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.toggle-card.modified {
  border-color: var(--accent);
}

.toggle-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
}

.toggle-card-label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.3;
}

.modified-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
  flex-shrink: 0;
}

.toggle-card-control {
  display: flex;
  align-items: center;
  gap: 8px;
}

.toggle-state {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-secondary);
}

.toggle-state.active {
  color: var(--accent);
}

.models-status {
  margin-bottom: 24px;
}

.models-status h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0 0 12px 0;
}

.models-grid {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
}

.model-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 14px 20px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  min-width: 280px;
}

.model-card.loaded {
  border-color: #2ecc71;
}

.model-indicator {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  flex-shrink: 0;
}

.indicator-green {
  background: #2ecc71;
  box-shadow: 0 0 8px rgba(46, 204, 113, 0.5);
}

.indicator-red {
  background: #e74c3c;
  box-shadow: 0 0 8px rgba(231, 76, 60, 0.5);
}

.model-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.model-name {
  font-size: 13px;
  font-weight: 600;
}

.model-status {
  font-size: 12px;
  color: var(--text-secondary);
}

.model-badge {
  font-size: 10px;
  font-weight: 700;
  padding: 3px 10px;
  border-radius: 4px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin-left: auto;
}

.badge-ok {
  color: #22c55e;
  background: rgba(34, 197, 94, 0.15);
}

.badge-ko {
  color: #ef4444;
  background: rgba(239, 68, 68, 0.15);
}

.models-unavailable {
  opacity: 0.6;
}

.models-empty {
  font-size: 13px;
  color: var(--text-secondary);
  padding: 12px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
}

/* ── Token styles ── */
/* Badge visible dans l'en-tête de la carte composant (la gestion CRUD est dans BotTokenManager.vue). */
.token-badge { font-size: 10px; font-weight: 600; padding: 2px 8px; border-radius: 4px; text-transform: uppercase; white-space: nowrap; }
.token-ok { color: #22c55e; background: rgba(34, 197, 94, 0.12); }
.token-missing { color: var(--text-secondary); background: rgba(255, 255, 255, 0.06); }
</style>
