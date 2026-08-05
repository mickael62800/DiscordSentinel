<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { parseBoolConfig } from "@/utils/configFlags";
import { botConfigService } from "@/services/botConfigService";
import { useToast } from "../../composables/useToast";
import { clampNumberValue } from "../../utils/clampNumber";
import type { BotDefinition, BotGuildConfig, ConfigField } from "../../types";
import AppToggle from "../atoms/AppToggle.vue";
import ConfigFieldRow from "../molecules/ConfigFieldRow.vue";

/**
 * Persistance de la configuration. Par defaut sentinel-api ; la plateforme
 * Nexus fournit la sienne, car c'est un autre backend (autre base, autre
 * passerelle). Le rendu du formulaire, lui, est identique : il est pilote par
 * `definition.config_schema`.
 */
export interface ConfigPersistence {
  set(guildId: string, botName: string, key: string, value: string): Promise<unknown>;
  remove(guildId: string, botName: string, key: string): Promise<unknown>;
}

const props = withDefaults(
  defineProps<{
    definition: BotDefinition;
    configs: BotGuildConfig[];
    guildId: string;
    persistence?: ConfigPersistence;
  }>(),
  { persistence: undefined },
);

const persist = computed<ConfigPersistence>(() => props.persistence ?? botConfigService);

const emit = defineEmits<{
  (e: "saved"): void;
}>();

const { success, error: showError } = useToast();

const formValues = ref<Record<string, string>>({});
const savedValues = ref<Record<string, string>>({});
const saving = ref(false);
const successMessage = ref("");

const isWorker = computed(() => props.definition.bot_name.endsWith("-worker"));

const configFields = computed<ConfigField[]>(() => {
  const schema = props.definition.config_schema;
  return Array.isArray(schema) ? schema : [];
});

const booleanFields = computed(() => configFields.value.filter((f) => f.type === "boolean"));
const numberFields = computed(() => configFields.value.filter((f) => f.type === "number"));
const channelFields = computed(() => configFields.value.filter((f) => f.type === "channel"));
const categoryFields = computed(() => configFields.value.filter((f) => f.type === "category"));
const roleFields = computed(() => configFields.value.filter((f) => f.type === "role"));
const enumFields = computed(() => configFields.value.filter((f) => f.type === "enum"));
const voiceFields = computed(() => configFields.value.filter((f) => f.type === "voice"));
const listFields = computed(() =>
  configFields.value.filter((f) =>
    f.type === "channel_list" || f.type === "role_list" || f.type === "voice_list"
    || f.type === "channel_schedule_list",
  ),
);

function isMultilineKey(k: string): boolean {
  return k.endsWith("_message") || k.endsWith("_multipliers");
}
const longTextFields = computed(() =>
  configFields.value.filter((f) => f.type === "text" && isMultilineKey(f.key)),
);
const shortTextFields = computed(() =>
  configFields.value.filter((f) => f.type === "text" && !isMultilineKey(f.key)),
);

/**
 * Types deja repartis dans une section nommee. Sert au filet de securite
 * ci-dessous : tout ce qui n'est pas ici doit quand meme s'afficher.
 */
const TYPES_CLASSES = new Set([
  "boolean", "number", "enum", "channel", "voice", "category", "role",
  "channel_list", "role_list", "voice_list", "channel_schedule_list", "text",
]);

/**
 * Champs d'un type qu'aucune section ne connait.
 *
 * Sans ce filet, un type ajoute cote base disparaissait PUREMENT du
 * formulaire : le reglage existait, le bot le lisait, mais personne ne
 * pouvait le renseigner et rien ne le signalait. C'est ce qui est arrive aux
 * lobbies vocaux — sans eux, le module ne savait pas quel salon declenche la
 * creation d'un vocal temporaire.
 */
const unclassifiedFields = computed(() =>
  configFields.value.filter((f) => !TYPES_CLASSES.has(f.type)),
);

const visibleSections = computed(() => {
  const all = [
    { title: "Valeurs", fields: numberFields.value, wide: false },
    { title: "Choix", fields: enumFields.value, wide: false },
    { title: "Salons", fields: channelFields.value, wide: false },
    { title: "Salons vocaux", fields: voiceFields.value, wide: false },
    { title: "Categories", fields: categoryFields.value, wide: false },
    { title: "Roles", fields: roleFields.value, wide: false },
    { title: "Listes", fields: listFields.value, wide: true },
    { title: "Textes courts", fields: shortTextFields.value, wide: false },
    { title: "Textes longs", fields: longTextFields.value, wide: true },
    { title: "Autres reglages", fields: unclassifiedFields.value, wide: true },
  ];
  return all.filter((s) => s.fields.length > 0);
});

const allTogglesOn = computed(() =>
  booleanFields.value.length > 0
  && booleanFields.value.every((f) => parseBoolConfig(formValues.value[f.key])),
);

function enableAllToggles() {
  for (const field of booleanFields.value) formValues.value[field.key] = "true";
}
function disableAllToggles() {
  for (const field of booleanFields.value) formValues.value[field.key] = "false";
}

function isFieldModified(key: string): boolean {
  return (formValues.value[key] ?? "") !== (savedValues.value[key] ?? "");
}

/**
 * Une cle est "disabled" quand son `depends_on.key` n'a pas la valeur
 * `equals` requise. Ex : tous les champs avec `depends_on:{key:"enabled",
 * equals:"true"}` sont grises tant que `enabled` est OFF.
 *
 * Cas speciaux :
 *  - `equals:""` (chaine vide) signifie "le parent a une valeur non-zero
 *    et non-vide" — utile pour les champs numeriques ou 0 = desactive
 *    (ex: scan interval depend de timeout > 0).
 *
 * Recursivite : si le parent est lui-meme disabled (chaine de depends_on
 * type enabled -> sub_feature_enabled -> sub_feature_channel_id),
 * l'enfant l'est aussi. On remonte la chaine jusqu'a un noeud sans
 * depends_on. Detection des cycles via un Set de cles deja visitees.
 */
function isFieldDisabled(field: ConfigField, visited: Set<string> = new Set()): boolean {
  const dep = field.depends_on as { key: string; equals: string } | undefined;
  if (!dep) return false;
  if (visited.has(field.key)) return false; // garde-fou cycle
  visited.add(field.key);

  // 1) check direct sur la valeur du parent
  const v = formValues.value[dep.key];
  let directlyDisabled: boolean;
  if (dep.equals === "true") directlyDisabled = !parseBoolConfig(v);
  else if (dep.equals === "false") directlyDisabled = parseBoolConfig(v);
  else if (dep.equals === "") directlyDisabled = v === undefined || v === "" || v === "0" || v === "false";
  else directlyDisabled = v !== dep.equals;

  if (directlyDisabled) return true;

  // 2) check transitif : si le parent est lui-meme disabled (cascade),
  // on l'est aussi.
  const parent = configFields.value.find((f) => f.key === dep.key);
  if (parent && isFieldDisabled(parent, visited)) return true;

  return false;
}

const hasChanges = computed(() =>
  configFields.value.some((f) => isFieldModified(f.key)),
);

const changesCount = computed(() =>
  configFields.value.filter((f) => isFieldModified(f.key)).length,
);

function fieldStatus(field: ConfigField): { text: string; source: "db" | "default" | "none" } {
  const dbValue = savedValues.value[field.key];

  if (isWorker.value) {
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

function loadFormValues() {
  const values: Record<string, string> = {};
  for (const cfg of props.configs.filter((c) => c.bot_name === props.definition.bot_name)) {
    values[cfg.config_key] = cfg.config_value;
  }
  savedValues.value = { ...values };
  formValues.value = { ...values };
}

function cancelChanges() {
  formValues.value = { ...savedValues.value };
}

async function save() {
  saving.value = true;
  successMessage.value = "";
  try {
    for (const field of configFields.value) {
      if (!isFieldModified(field.key)) continue;
      let value = formValues.value[field.key] ?? "";
      if (field.type === "number" && value) {
        value = clampNumberValue(value, field.min, field.max);
        formValues.value[field.key] = value;
      }
      if (value) {
        await persist.value.set(props.guildId, props.definition.bot_name, field.key, String(value));
      } else {
        await persist.value.remove(props.guildId, props.definition.bot_name, field.key);
      }
    }
    const count = changesCount.value;
    successMessage.value = `${count} parametre(s) enregistre(s)`;
    success(`${count} parametre(s) enregistre(s)`);
    emit("saved");
    setTimeout(() => (successMessage.value = ""), 3000);
  } catch (e) {
    console.error("Erreur sauvegarde:", e);
    showError("Erreur lors de la sauvegarde de la configuration");
  } finally {
    saving.value = false;
  }
}

// Recharge les valeurs quand le composant selectionne ou les configs changent
watch(() => [props.definition.bot_name, props.configs], loadFormValues, { immediate: true });
</script>

<template>
  <div class="config-form">
    <div class="config-form-header">
      <h2>{{ definition.display_name }}</h2>
    </div>

    <div v-if="configFields.length === 0" class="no-params">
      Ce composant n'a pas de parametres configurables par serveur.
    </div>

    <template v-else>
      <!-- Notice worker : reglages lus au demarrage uniquement -->
      <div v-if="isWorker" class="worker-notice">
        Les réglages des workers sont lus au démarrage — un changement prend
        effet au prochain redémarrage du worker.
      </div>

      <!-- Section toggles -->
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
            :class="{ modified: isFieldModified(field.key), 'field-disabled': isFieldDisabled(field) }"
            :title="isFieldDisabled(field) ? 'Depend d\'une autre option desactivee' : undefined"
          >
            <div class="toggle-card-header">
              <span class="toggle-card-label" :title="field.label">{{ field.label }}</span>
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

      <!-- Sections non-boolean -->
      <div class="sections-flow">
        <div
          v-for="section in visibleSections"
          :key="section.title"
          class="inputs-section"
          :class="[
            section.fields.length >= 4 || section.wide ? 'section-full' : 'section-auto',
            section.wide ? 'section-textareas' : '',
          ]"
          :style="
            !section.wide && section.fields.length < 4
              ? { flexGrow: section.fields.length }
              : undefined
          "
        >
          <h3 class="section-title">{{ section.title }}</h3>
          <div class="fields-grid-2col">
            <ConfigFieldRow
              v-for="field in section.fields"
              :key="field.key"
              :field="field"
              :model-value="formValues[field.key] ?? ''"
              :guild-id="guildId"
              :modified="isFieldModified(field.key)"
              :hint="fieldStatus(field).text"
              :hint-source="fieldStatus(field).source"
              :disabled="isFieldDisabled(field)"
              @update:model-value="formValues[field.key] = $event"
            />
          </div>
        </div>
      </div>

      <div class="form-actions">
        <button class="btn-save" :disabled="saving || !hasChanges" @click="save">
          {{ saving ? "Enregistrement..." : hasChanges ? `Enregistrer (${changesCount})` : "Aucune modification" }}
        </button>
        <button v-if="hasChanges" class="btn-cancel" @click="cancelChanges">Annuler</button>
        <span v-if="successMessage" class="success-msg">{{ successMessage }}</span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.config-form {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: 24px;
}

.config-form-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 20px;
}

.config-form-header h2 { font-size: 18px; color: var(--text-primary); }

.no-params { color: var(--text-secondary); font-size: 14px; padding: 20px 0; }

/* Notice info discrete pour les workers (config figee au demarrage). */
.worker-notice {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
  color: var(--text-secondary);
  background: rgba(99, 102, 241, 0.06);
  border: 1px solid var(--border);
  border-left: 3px solid var(--accent);
  border-radius: var(--radius-sm);
  padding: 10px 14px;
  margin-bottom: 16px;
  line-height: 1.4;
}

/* Tooltip */
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
  width: 16px; height: 16px;
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
  border-radius: var(--radius-sm);
  font-size: 12px;
  font-weight: 400;
  font-style: normal;
  white-space: normal;
  width: max-content;
  max-width: 280px;
  box-shadow: var(--shadow-md);
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
.tooltip-wrap:hover .tooltip-text { display: block; }

/* Toggles */
.toggles-section {}
.section-title-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin: 24px 0 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border);
}
.toggles-section:first-child .section-title-row { margin-top: 0; }
.section-title {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0;
}
.btn-toggle-all {
  padding: 5px 14px;
  border: 1px solid var(--accent);
  border-radius: var(--radius-sm);
  background: rgba(99, 102, 241, 0.08);
  color: var(--accent);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}
.btn-toggle-all:hover { background: var(--accent); color: white; }

.toggles-grid {
  display: grid;
  /* 1 col mobile, ~2 tablette, 3 colonnes max sur desktop : cartes larges,
     labels complets (multi-lignes) et lisibles sans survol. */
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 8px;
  margin-top: 8px;
}
.toggles-grid > .toggle-card { min-width: 0; }
@media (min-width: 1400px) {
  .toggles-grid { grid-template-columns: repeat(3, minmax(0, 1fr)); }
}
@media (min-width: 1900px) {
  .toggles-grid { grid-template-columns: repeat(4, minmax(0, 1fr)); }
}

.toggle-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 10px 12px;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.toggle-card.modified { border-color: var(--accent); }
.toggle-card.field-disabled {
  opacity: 0.45;
  pointer-events: none;
  filter: grayscale(0.4);
}
.toggle-card-header { display: flex; align-items: center; justify-content: space-between; gap: 4px; }
.toggle-card-label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  line-height: 1.35;
  /* Label complet : on autorise le retour a la ligne plutot que de
     tronquer en ellipsis (illisible sans survol). */
  white-space: normal;
  overflow-wrap: anywhere;
  min-width: 0;
  flex: 1 1 auto;
}
.modified-dot {
  width: 6px; height: 6px;
  border-radius: 50%;
  background: var(--accent);
  flex-shrink: 0;
}
.toggle-card-control { display: flex; align-items: center; gap: 8px; }
.toggle-state { font-size: 11px; font-weight: 700; color: var(--text-secondary); }
.toggle-state.active { color: var(--accent); }

/* Sections form */
.sections-flow {
  display: flex;
  flex-wrap: wrap;
  gap: 12px;
  margin-top: 12px;
}
.inputs-section {
  margin: 0;
  display: flex;
  flex-direction: column;
}
.inputs-section.section-full { flex: 1 1 100%; min-width: 0; }
.inputs-section.section-auto { flex: 1 1 360px; min-width: 0; max-width: 100%; }

.fields-grid-2col {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 12px;
  margin-top: 8px;
  margin-bottom: 12px;
  flex: 1;
}
@media (min-width: 1900px) {
  /* Plafonne a 3 colonnes meme sur tres grand ecran : champs lisibles
     plutot que minuscules. */
  .fields-grid-2col { grid-template-columns: repeat(3, 1fr); }
}

.inputs-section.section-textareas .fields-grid-2col {
  grid-template-columns: repeat(auto-fit, minmax(420px, 1fr));
}
@media (min-width: 1900px) {
  .inputs-section.section-textareas .fields-grid-2col {
    grid-template-columns: repeat(3, 1fr);
  }
}

.inputs-section.section-auto .fields-grid-2col {
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
}

/* Form actions */
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
  border-radius: var(--radius-md);
  font-size: 14px;
  font-weight: 600;
  cursor: pointer;
}
.btn-save:hover:not(:disabled) { opacity: 0.9; }
.btn-save:disabled { opacity: 0.4; cursor: not-allowed; }

.btn-cancel {
  padding: 10px 20px;
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  font-size: 14px;
  cursor: pointer;
}
.btn-cancel:hover { border-color: var(--danger); color: var(--danger); }

.success-msg { color: var(--success); font-size: 13px; font-weight: 500; }

@media (max-width: 640px) {
  [class*="-fields"],
  [class*="-grid"] {
    grid-template-columns: 1fr !important;
    gap: 10px !important;
  }
}
</style>
