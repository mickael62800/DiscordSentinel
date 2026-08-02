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

/// Recharge les curseurs depuis les valeurs REELLES de la regle.
///
/// Elles etaient auparavant deduites de `action` via une table figee, sans
/// jamais lire ce qui etait enregistre. Comme ce reset se declenche a chaque
/// changement du prop, un enregistrement reussi etait aussitot ecrase par la
/// valeur inventee : on saisissait 3.5, l'API acceptait, et l'ecran
/// reaffichait 2.
function resetFromRule() {
  const r = props.rule;
  weight.value = r.weight;
  thresholdWarn.value = r.threshold_warn;
  thresholdDelete.value = r.threshold_delete;
  thresholdMute.value = r.threshold_mute;
  thresholdBan.value = r.threshold_ban;
  baseline.value = {
    weight: r.weight,
    warn: r.threshold_warn,
    del: r.threshold_delete,
    mute: r.threshold_mute,
    ban: r.threshold_ban,
  };
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
  <div :class="['card', 'rule-card', { disabled: !rule.enabled, dirty }]">
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
  transition: opacity var(--transition-base), border-color var(--transition-fast);
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
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
  gap: var(--space-sm);
  flex-wrap: wrap;
}

.rule-title {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
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
  gap: var(--space-md);
  padding: var(--space-md);
  border-radius: var(--radius-md);
  background-color: var(--bg-primary);
  border: 1px solid var(--border);
}

.param-row {
  display: flex;
  flex-direction: column;
  gap: var(--space-xs);
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
  gap: var(--space-sm);
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
  border-radius: var(--radius-sm);
  background-color: var(--bg-hover);
}

.range-value.warn { color: var(--info); }
.range-value.delete { color: var(--warning); }
.range-value.mute { color: var(--severity-mute); }
.range-value.ban { color: var(--danger); }

.separator {
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
  opacity: 0.7;
  padding-top: var(--space-xs);
  border-top: 1px dashed var(--border);
}

.thresholds-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: var(--space-sm) 14px;
}

/* Breakpoint --bp-sm (480px) : stack vertical sur mobile landscape */
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
  gap: var(--space-sm);
  flex-wrap: wrap;
}

.actions {
  display: inline-flex;
  gap: var(--space-xs);
  flex-wrap: wrap;
}

.save-btn {
  background-color: var(--accent);
  color: white;
  border: 1px solid var(--accent);
  font-size: 12px;
  font-weight: 600;
  padding: 5px 14px;
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: opacity var(--transition-fast);
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
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all var(--transition-fast);
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
