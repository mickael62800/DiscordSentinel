<script setup lang="ts">
import { computed } from "vue";
import type { ConfigField } from "@/types";
import FieldDescription from "../atoms/FieldDescription.vue";
import NumberInputWithUnit from "../atoms/NumberInputWithUnit.vue";
import EnumSelect from "../atoms/EnumSelect.vue";
import ChannelSelect from "../atoms/ChannelSelect.vue";
import VoiceChannelSelect from "../atoms/VoiceChannelSelect.vue";
import CategorySelect from "../atoms/CategorySelect.vue";
import RoleSelect from "../atoms/RoleSelect.vue";
import IdMultiplierMapField from "./IdMultiplierMapField.vue";
import IdsListPickerField from "./IdsListPickerField.vue";
import ChannelScheduleEditor from "./ChannelScheduleEditor.vue";

// Champs config qui stockent un mapping "id:valeur" en lignes/CSV. On
// remplace le textarea brut par un picker (channel ou role) + valeur +
// liste des entrees avec nom resolu et bouton supprimer.
const CHANNEL_MAP_KEYS = new Set<string>([
  "xp_channel_multipliers",
  "channel_thresholds",
  "vision_channel_thresholds",
]);
const ROLE_MAP_KEYS = new Set<string>(["xp_role_multipliers", "temp_roles"]);

// Champs config qui stockent une liste plate d'IDs en CSV (sans valeur
// associee). On remplace le textarea brut par un multi-picker + chips.
const CHANNEL_LIST_KEYS = new Set<string>([
  "ignored_channels",
  "excluded_channels",
  "whitelist_channels",
  "exempt_channels",
  "command_channels",
  // Liste de salons VOCAUX a observer pour les logs (voice-bot).
  "observed_voice_channels",
]);

// Sous-ensemble de CHANNEL_LIST_KEYS dont le picker doit lister des salons
// VOCAUX (et non textuels).
const VOICE_CHANNEL_LIST_KEYS = new Set<string>(["observed_voice_channels"]);
const ROLE_LIST_KEYS = new Set<string>([
  "ignored_roles",
  "excluded_roles",
  "whitelist_roles",
  "exempt_roles",
  "double_xp_roles",
  // Progression : roles exclus du classement mensuel.
  "monthly_ranking_excluded_roles",
  // Welcome : "roles apres validation du reglement" -> liste de roles.
  "rules_role_id",
]);

// Champs type="channel" qui doivent lister des salons VOCAUX (et non
// textuels) : salons lobby creators + salon AFK du voice-bot. Sans ca, le
// dropdown affiche les salons textuels, inutilisables ici.
const VOICE_CHANNEL_KEYS = new Set<string>([
  "public_creator_channel_id",
  "private_creator_channel_id",
  "game_creator_channel_id",
  "afk_channel_id",
  // Compteur de membres : salon VOCAL (seul a accepter espaces/majuscules/":"
  // dans le nom, ex "Membres : 515").
  "counter_channel_id",
  // Compteur de membres connectes EN VOCAL : renomme aussi un salon vocal.
  "voice_counter_channel_id",
]);

const props = defineProps<{
  field: ConfigField;
  modelValue: string;
  guildId: string | null;
  modified?: boolean;
  hint?: string;
  hintSource?: string;
  /** Si true, le champ est inactif (dependance non satisfaite). */
  disabled?: boolean;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

function update(v: string) {
  emit("update:modelValue", v);
}

const isMultilineText = computed(
  () => props.field.type === "text" && props.field.key.endsWith("_message"),
);

// Selection du composant : le TYPE du schema decide (source de verite). Les
// Set de cles ci-dessus ne sont qu'un REPLI pour les champs historiques encore
// declares en "text"/"channel" ; toute nouvelle cle doit simplement declarer
// son type ("voice", "channel_list", "voice_list", "role_list") et fonctionne
// sans toucher a ce fichier.
const isChannelMap = computed(
  () => props.field.type === "text" && CHANNEL_MAP_KEYS.has(props.field.key),
);
const isRoleMap = computed(
  () => props.field.type === "text" && ROLE_MAP_KEYS.has(props.field.key),
);
const isChannelList = computed(
  () =>
    props.field.type === "channel_list" ||
    (props.field.type === "text" && CHANNEL_LIST_KEYS.has(props.field.key)),
);
const isRoleList = computed(
  () =>
    props.field.type === "role_list" ||
    (props.field.type === "text" && ROLE_LIST_KEYS.has(props.field.key)),
);
const isVoiceChannelList = computed(
  () =>
    props.field.type === "voice_list" ||
    (props.field.type === "text" && VOICE_CHANNEL_LIST_KEYS.has(props.field.key)),
);
const isVoiceChannel = computed(
  () =>
    props.field.type === "voice" ||
    (props.field.type === "channel" && VOICE_CHANNEL_KEYS.has(props.field.key)),
);

const mapDefaults = computed(() => {
  switch (props.field.key) {
    case "xp_channel_multipliers":
    case "xp_role_multipliers":
      return { label: "Multiplicateur", step: 0.25, min: 0, default: 1 };
    case "channel_thresholds":
    case "vision_channel_thresholds":
      return { label: "Seuil", step: 0.05, min: 0, max: 1, default: 0.7 };
    case "temp_roles":
      return { label: "Durée (s)", step: 60, min: 1, default: 3600 };
    default:
      return { label: "Valeur", step: 1, default: 1 };
  }
});
</script>

<template>
  <div class="field-row" :class="{ modified, 'field-disabled': disabled }"
    :title="disabled ? 'Cette option depend d\'une autre desactivee' : undefined">
    <!-- 1. Label + hint a droite -->
    <div class="field-label-row">
      <label :for="field.key" class="field-label">
        {{ field.label }}
        <span v-if="modified" class="modified-dot" />
        <span
          v-if="field.restart_required"
          class="restart-badge"
          title="Ce reglage n'est lu qu'au demarrage : la modification prend effet apres un redemarrage."
        >&#8635; redémarrage requis</span>
      </label>
      <span
        v-if="hint"
        class="field-hint-inline"
        :class="hintSource ? `hint-${hintSource}` : ''"
      >
        {{ hint }}
      </span>
    </div>

    <!-- 2. Description pedagogique juste sous le label -->
    <FieldDescription v-if="field.description" :text="field.description" />

    <!-- 3. Input -->
    <NumberInputWithUnit
      v-if="field.type === 'number'"
      :id="field.key"
      :model-value="modelValue"
      :unit="field.unit"
      :min="field.min"
      :max="field.max"
      :placeholder="field.default !== undefined ? String(field.default) : '0'"
      @update:model-value="update"
    />

    <EnumSelect
      v-else-if="field.type === 'enum' && field.options"
      :id="field.key"
      :model-value="modelValue"
      :options="field.options"
      :placeholder="field.default !== undefined ? String(field.default) : '—'"
      @update:model-value="update"
    />

    <VoiceChannelSelect
      v-else-if="isVoiceChannel"
      :id="field.key"
      :model-value="modelValue"
      :guild-id="guildId"
      @update:model-value="update"
    />

    <ChannelSelect
      v-else-if="field.type === 'channel'"
      :id="field.key"
      :model-value="modelValue"
      :guild-id="guildId"
      @update:model-value="update"
    />

    <CategorySelect
      v-else-if="field.type === 'category'"
      :id="field.key"
      :model-value="modelValue"
      :guild-id="guildId"
      @update:model-value="update"
    />

    <RoleSelect
      v-else-if="field.type === 'role'"
      :id="field.key"
      :model-value="modelValue"
      :guild-id="guildId"
      @update:model-value="update"
    />

    <IdsListPickerField
      v-else-if="isChannelList || isRoleList || isVoiceChannelList"
      :model-value="modelValue"
      :guild-id="guildId"
      :kind="isRoleList ? 'role' : (isVoiceChannelList ? 'channel-voice' : 'channel')"
      @update:model-value="update"
    />

    <ChannelScheduleEditor
      v-else-if="field.type === 'channel_schedule_list'"
      :model-value="modelValue"
      :guild-id="guildId"
      @update:model-value="update"
    />

    <IdMultiplierMapField
      v-else-if="isChannelMap || isRoleMap"
      :model-value="modelValue"
      :guild-id="guildId"
      :kind="
        isRoleMap
          ? 'role'
          : field.key === 'xp_channel_multipliers'
            ? 'channel-all'
            : 'channel'
      "
      :value-label="mapDefaults.label"
      :value-step="mapDefaults.step"
      :value-min="mapDefaults.min"
      :value-max="mapDefaults.max"
      :value-default="mapDefaults.default"
      @update:model-value="update"
    />

    <textarea
      v-else-if="isMultilineText"
      :id="field.key"
      :value="modelValue"
      class="field-input field-textarea"
      rows="6"
      :placeholder="field.default !== undefined ? String(field.default) : ''"
      @input="update(($event.target as HTMLTextAreaElement).value)"
    />

    <input
      v-else
      :id="field.key"
      :value="modelValue"
      type="text"
      class="field-input"
      :placeholder="field.default !== undefined ? String(field.default) : ''"
      @input="update(($event.target as HTMLInputElement).value)"
    />
  </div>
</template>

<style scoped>
.field-row {
  /* Cellule verticale : label-hint / description / input empiles.
     L input est toujours colle en bas pour que toutes les cellules d une
     meme ligne aient leurs inputs alignes (peu importe la longueur de la
     description). */
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 12px 14px;
  background: var(--bg-card, #1a1d24);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  min-width: 0;
  /* h-full implicite : le grid parent stretch les cellules a la hauteur
     de la plus grande, on doit donc pouvoir pousser l input en bas. */
  height: 100%;
}

/* Pousse le dernier enfant (= l input) tout en bas de la cellule.
   Comme la cellule est en flex column, margin-top: auto sur le dernier
   absorbe l espace libre. */
.field-row > *:last-child {
  margin-top: auto;
}

.field-row.modified {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}

.field-row.field-disabled {
  opacity: 0.45;
  pointer-events: none;
  filter: grayscale(0.4);
}

.field-label-row {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 12px;
  min-width: 0;
}

.field-label {
  display: flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  font-weight: 600;
  color: var(--text-primary);
  text-transform: none;
  flex-shrink: 0;
}

.field-hint-inline {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: monospace;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  text-align: right;
  min-width: 0;
}

.field-hint-inline.hint-db {
  color: var(--success, var(--success));
}

.field-hint-inline.hint-default {
  color: var(--text-secondary);
}

.field-hint-inline.hint-none {
  color: var(--text-secondary);
  opacity: 0.6;
}

.modified-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent);
}

/* Badge discret signalant qu'un reglage n'est applique qu'apres
   redemarrage (champ fige au demarrage du composant). Teinte ambre. */
.restart-badge {
  font-size: 10px;
  font-weight: 600;
  color: var(--warning, #f0a020);
  border: 1px solid var(--warning, #f0a020);
  border-radius: var(--radius-sm);
  padding: 1px 6px;
  white-space: nowrap;
  opacity: 0.85;
}

.field-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
}

.field-input:focus {
  outline: none;
  border-color: var(--accent);
}

.field-textarea {
  resize: vertical;
  min-height: 100px;
  font-family: inherit;
}

</style>
