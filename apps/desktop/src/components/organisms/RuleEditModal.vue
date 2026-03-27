<script setup lang="ts">
import { ref, watch } from "vue";
import type { ModerationRule, UpdateRuleParams } from "../../types";
import AppButton from "../atoms/AppButton.vue";
import AppToggle from "../atoms/AppToggle.vue";

const props = defineProps<{
  rule: ModerationRule;
}>();

const emit = defineEmits<{
  save: [params: UpdateRuleParams];
  close: [];
}>();

const guildId = ref("default");
const weight = ref(3.0);
const thresholdWarn = ref(2.0);
const thresholdDelete = ref(4.0);
const thresholdMute = ref(6.0);
const thresholdBan = ref(9.0);
const enabled = ref(true);
const saving = ref(false);
const error = ref<string | null>(null);

// Map rule_type to flag_type for the API
function ruleTypeToFlagType(ruleType: string): string {
  switch (ruleType) {
    case "rate_limit": return "spam";
    case "content_filter": return "insult";
    case "join_rate": return "spam";
    default: return ruleType;
  }
}

// Set defaults based on rule action
function defaultsForAction(action: string) {
  switch (action) {
    case "warn": return { weight: 2.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 };
    case "delete": return { weight: 3.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 };
    case "mute": return { weight: 5.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 };
    case "ban": case "lockdown": return { weight: 7.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 };
    default: return { weight: 3.0, warn: 2.0, del: 4.0, mute: 6.0, ban: 9.0 };
  }
}

watch(() => props.rule, (r) => {
  const d = defaultsForAction(r.action);
  weight.value = d.weight;
  thresholdWarn.value = d.warn;
  thresholdDelete.value = d.del;
  thresholdMute.value = d.mute;
  thresholdBan.value = d.ban;
  enabled.value = r.enabled;
}, { immediate: true });

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
    guild_id: guildId.value,
    flag_type: ruleTypeToFlagType(props.rule.rule_type),
    weight: weight.value,
    threshold_warn: thresholdWarn.value,
    threshold_delete: thresholdDelete.value,
    threshold_mute: thresholdMute.value,
    threshold_ban: thresholdBan.value,
    enabled: enabled.value,
  });

  saving.value = false;
}
</script>

<template>
  <div class="modal-overlay" @click.self="emit('close')">
    <div class="modal">
      <div class="modal-header">
        <h2>Edit Rule: {{ rule.name }}</h2>
        <button class="close-btn" @click="emit('close')">&times;</button>
      </div>

      <div class="modal-body">
        <div class="field">
          <label>Guild ID</label>
          <input v-model="guildId" type="text" placeholder="defaut" />
          <span class="hint">Discord server ID this rule applies to</span>
        </div>

        <div class="field">
          <label>Weight</label>
          <div class="range-row">
            <input v-model.number="weight" type="range" min="0" max="10" step="0.5" />
            <span class="range-value">{{ weight }}</span>
          </div>
          <span class="hint">Contribution de ce flag au score total</span>
        </div>

        <div class="separator">Seuils (doivent etre strictement croissants)</div>

        <div class="thresholds-grid">
          <div class="field">
            <label>Warn</label>
            <div class="range-row">
              <input v-model.number="thresholdWarn" type="range" min="0" max="10" step="0.5" />
              <span class="range-value warn">{{ thresholdWarn }}</span>
            </div>
          </div>

          <div class="field">
            <label>Delete</label>
            <div class="range-row">
              <input v-model.number="thresholdDelete" type="range" min="0" max="10" step="0.5" />
              <span class="range-value delete">{{ thresholdDelete }}</span>
            </div>
          </div>

          <div class="field">
            <label>Mute</label>
            <div class="range-row">
              <input v-model.number="thresholdMute" type="range" min="0" max="15" step="0.5" />
              <span class="range-value mute">{{ thresholdMute }}</span>
            </div>
          </div>

          <div class="field">
            <label>Ban</label>
            <div class="range-row">
              <input v-model.number="thresholdBan" type="range" min="0" max="20" step="0.5" />
              <span class="range-value ban">{{ thresholdBan }}</span>
            </div>
          </div>
        </div>

        <div class="field toggle-field">
          <label>Enabled</label>
          <AppToggle v-model="enabled" />
        </div>

        <p v-if="error" class="error-msg">{{ error }}</p>
      </div>

      <div class="modal-footer">
        <AppButton variant="secondary" @click="emit('close')">Annuler</AppButton>
        <AppButton variant="primary" :disabled="saving" @click="handleSave">
          {{ saving ? "Sauvegarde..." : "Sauvegarder" }}
        </AppButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 200;
}

.modal {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  width: 520px;
  max-height: 90vh;
  overflow-y: auto;
  box-shadow: 0 16px 48px rgba(0, 0, 0, 0.4);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 24px 24px 16px;
  border-bottom: 1px solid var(--border);
}

.modal-header h2 {
  font-size: 18px;
  font-weight: 600;
}

.close-btn {
  background: none;
  color: var(--text-secondary);
  font-size: 22px;
  padding: 4px 8px;
  line-height: 1;
}

.close-btn:hover {
  color: var(--text-primary);
}

.modal-body {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
}

.field input[type="text"] {
  width: 100%;
}

.hint {
  font-size: 11px;
  color: var(--text-secondary);
  opacity: 0.7;
}

.range-row {
  display: flex;
  align-items: center;
  gap: 12px;
}

.range-row input[type="range"] {
  flex: 1;
  accent-color: var(--accent);
  height: 6px;
  cursor: pointer;
}

.range-value {
  min-width: 36px;
  text-align: center;
  font-weight: 700;
  font-size: 14px;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  padding: 4px 8px;
  border-radius: 6px;
  background-color: var(--bg-hover);
}

.range-value.warn { color: var(--info); }
.range-value.delete { color: var(--warning); }
.range-value.mute { color: #ff9500; }
.range-value.ban { color: var(--danger); }

.separator {
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
  padding-top: 4px;
}

.thresholds-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}

.toggle-field {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
}

.error-msg {
  color: var(--danger);
  font-size: 13px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 8px;
  padding: 16px 24px;
  border-top: 1px solid var(--border);
}

.modal-footer button.secondary {
  background-color: transparent;
  border: 1px solid var(--border);
  color: var(--text-secondary);
}

.modal-footer button.secondary:hover {
  border-color: var(--text-primary);
  color: var(--text-primary);
}
</style>
