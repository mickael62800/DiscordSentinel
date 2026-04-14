<script setup lang="ts">
import { ref, computed, watch } from "vue";
import type { ModerationRule, UpdateRuleParams } from "../../types";
import AppBadge from "../atoms/AppBadge.vue";
import AppToggle from "../atoms/AppToggle.vue";
import { actionVariant, actionLabel, typeLabel } from "../../utils/variants";

const props = defineProps<{
  rule: ModerationRule;
  guildId: string;
}>();

const emit = defineEmits<{
  toggle: [rule: ModerationRule];
  save: [params: UpdateRuleParams];
}>();

function ruleName(rule: ModerationRule): string {
  return typeLabel(rule.rule_type);
}

// Map rule_type to flag_type pour l'API.
function ruleTypeToFlagType(ruleType: string): string {
  switch (ruleType) {
    case "rate_limit":
      return "spam";
    case "content_filter":
      return "insult";
    case "join_rate":
      return "spam";
    default:
      return ruleType;
  }
}

// Valeurs par defaut selon l'action configuree.
function defaultsForAction(action: string) {
  switch (action) {
    case "warn":
      return { weight: 2.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 };
    case "delete":
      return { weight: 3.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 };
    case "mute":
      return { weight: 5.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 };
    case "ban":
    case "lockdown":
      return { weight: 7.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 };
    default:
      return { weight: 3.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 };
  }
}

// Etat local des parametres (edition inline).
const weight = ref(3.0);
const thresholdWarn = ref(2.0);
const thresholdDelete = ref(4.0);
const thresholdMute = ref(6.0);
const thresholdBan = ref(9.0);
const saving = ref(false);
const error = ref<string | null>(null);

// Snapshot "baseline" pour detecter les changements non sauvegardes.
const baseline = ref({ weight: 3.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 });

function resetFromRule() {
  const d = defaultsForAction(props.rule.action);
  weight.value = d.weight;
  thresholdWarn.value = d.warn;
  thresholdDelete.value = d.del;
  thresholdMute.value = d.mute;
  thresholdBan.value = d.ban;
  baseline.value = { ...d };
  error.value = null;
}

watch(() => props.rule, resetFromRule, { immediate: true });

const dirty = computed(() => {
  return (
    weight.value !== baseline.value.weight ||
    thresholdWarn.value !== baseline.value.warn ||
    thresholdDelete.value !== baseline.value.del ||
    thresholdMute.value !== baseline.value.mute ||
    thresholdBan.value !== baseline.value.ban
  );
});

function validate(): string | null {
  if (weight.value < 0) return "Le poids doit etre >= 0";
  if (thresholdWarn.value >= thresholdDelete.value) return "warn doit etre < delete";
  if (thresholdDelete.value >= thresholdMute.value) return "delete doit etre < mute";
  if (thresholdMute.value >= thresholdBan.value) return "mute doit etre < ban";
  return null;
}

async function handleSave() {
  const validationError = validate();
  if (validationError) {
    error.value = validationError;
    return;
  }
  saving.value = true;
  error.value = null;
  emit("save", {
    guild_id: props.guildId,
    flag_type: ruleTypeToFlagType(props.rule.rule_type),
    weight: weight.value,
    threshold_warn: thresholdWarn.value,
    threshold_delete: thresholdDelete.value,
    threshold_mute: thresholdMute.value,
    threshold_ban: thresholdBan.value,
    enabled: props.rule.enabled,
  });
  // Update baseline pour ne plus etre "dirty" apres le save.
  baseline.value = {
    weight: weight.value,
    warn: thresholdWarn.value,
    del: thresholdDelete.value,
    mute: thresholdMute.value,
    ban: thresholdBan.value,
  };
  saving.value = false;
}

function handleReset() {
  weight.value = baseline.value.weight;
  thresholdWarn.value = baseline.value.warn;
  thresholdDelete.value = baseline.value.del;
  thresholdMute.value = baseline.value.mute;
  thresholdBan.value = baseline.value.ban;
  error.value = null;
}
</script>

<template>
  <div :class="['rule-card', { disabled: !rule.enabled, dirty }]">
    <div class="rule-header">
      <div class="rule-title">
        <h3>{{ ruleName(rule) }}</h3>
        <AppBadge :label="actionLabel(rule.action)" :variant="actionVariant(rule.action)" />
      </div>
      <AppToggle :model-value="rule.enabled" @update:model-value="emit('toggle', rule)" />
    </div>
    <p class="rule-description">{{ rule.description.replace(/\s*pour le serveur \d+/i, '') }}</p>

    <!-- Parametres inline -->
    <div class="params">
      <div class="param-row">
        <label>Poids</label>
        <div class="range-row">
          <input v-model.number="weight" type="range" min="0" max="10" step="0.5" />
          <span class="range-value">{{ weight }}</span>
        </div>
      </div>

      <div class="separator">Seuils (doivent etre strictement croissants)</div>

      <div class="thresholds-grid">
        <div class="param-row compact">
          <label>Avertissement</label>
          <div class="range-row">
            <input v-model.number="thresholdWarn" type="range" min="0" max="10" step="0.5" />
            <span class="range-value warn">{{ thresholdWarn }}</span>
          </div>
        </div>
        <div class="param-row compact">
          <label>Suppression</label>
          <div class="range-row">
            <input v-model.number="thresholdDelete" type="range" min="0" max="10" step="0.5" />
            <span class="range-value delete">{{ thresholdDelete }}</span>
          </div>
        </div>
        <div class="param-row compact">
          <label>Sourdine</label>
          <div class="range-row">
            <input v-model.number="thresholdMute" type="range" min="0" max="15" step="0.5" />
            <span class="range-value mute">{{ thresholdMute }}</span>
          </div>
        </div>
        <div class="param-row compact">
          <label>Bannissement</label>
          <div class="range-row">
            <input v-model.number="thresholdBan" type="range" min="0" max="20" step="0.5" />
            <span class="range-value ban">{{ thresholdBan }}</span>
          </div>
        </div>
      </div>

      <p v-if="error" class="error-msg">{{ error }}</p>
    </div>

    <div class="rule-footer">
      <AppBadge :label="typeLabel(rule.rule_type)" variant="default" />
      <div class="actions">
        <button v-if="dirty" class="reset-btn" :disabled="saving" @click="handleReset">
          Annuler
        </button>
        <button
          class="save-btn"
          :disabled="saving || !dirty"
          @click="handleSave"
        >
          {{ saving ? "Sauvegarde…" : dirty ? "Sauvegarder" : "A jour" }}
        </button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.rule-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 20px;
  transition: opacity 0.2s, border-color 0.15s;
  display: flex;
  flex-direction: column;
  gap: 12px;
  min-width: 0; /* evite qu'un contenu long force l'expansion au-dela de la col */
}

.rule-card.disabled {
  opacity: 0.6;
}

.rule-card.dirty {
  border-color: var(--accent);
  box-shadow: 0 0 0 1px var(--accent);
}

.rule-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 10px;
  flex-wrap: wrap;
}

.rule-title {
  display: flex;
  align-items: center;
  gap: 10px;
  flex: 1;
  min-width: 0;
  flex-wrap: wrap;
}

.rule-title h3 {
  font-size: 16px;
  font-weight: 600;
  margin: 0;
  overflow-wrap: anywhere;
  min-width: 0;
}

.rule-description {
  color: var(--text-secondary);
  font-size: 13px;
  margin: 0;
}

.params {
  display: flex;
  flex-direction: column;
  gap: 12px;
  padding: 12px;
  border-radius: 8px;
  background-color: var(--bg-primary);
  border: 1px solid var(--border);
}

.param-row {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.param-row label {
  font-size: 11px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.range-row {
  display: flex;
  align-items: center;
  gap: 10px;
}

.range-row input[type="range"] {
  flex: 1;
  accent-color: var(--accent);
  height: 4px;
  cursor: pointer;
  min-width: 0;
}

.range-value {
  min-width: 34px;
  text-align: center;
  font-weight: 700;
  font-size: 12px;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  padding: 3px 6px;
  border-radius: 4px;
  background-color: var(--bg-hover);
}

.range-value.warn { color: var(--info); }
.range-value.delete { color: var(--warning); }
.range-value.mute { color: #ff9500; }
.range-value.ban { color: var(--danger); }

.separator {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
  opacity: 0.7;
  padding-top: 4px;
  border-top: 1px dashed var(--border);
}

.thresholds-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 10px 14px;
}

@media (max-width: 480px) {
  .thresholds-grid {
    grid-template-columns: 1fr;
  }
}

.error-msg {
  color: var(--danger);
  font-size: 12px;
  margin: 0;
}

.rule-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 10px;
  flex-wrap: wrap;
}

.actions {
  display: inline-flex;
  gap: 6px;
  flex-wrap: wrap;
}

.save-btn {
  background-color: var(--accent);
  color: white;
  border: 1px solid var(--accent);
  font-size: 12px;
  font-weight: 600;
  padding: 5px 14px;
  border-radius: 6px;
  cursor: pointer;
  transition: opacity 0.15s;
}

.save-btn:disabled {
  background-color: transparent;
  color: var(--text-secondary);
  border-color: var(--border);
  cursor: not-allowed;
}

.save-btn:hover:not(:disabled) {
  opacity: 0.85;
}

.reset-btn {
  background: transparent;
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  padding: 5px 14px;
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.15s;
}

.reset-btn:hover:not(:disabled) {
  color: var(--text-primary);
  border-color: var(--text-primary);
}

.reset-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
