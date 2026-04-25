<script setup lang="ts">
import { computed } from "vue";
import type { ConfigField } from "@/types";
import FieldDescription from "../atoms/FieldDescription.vue";
import NumberInputWithUnit from "../atoms/NumberInputWithUnit.vue";
import EnumSelect from "../atoms/EnumSelect.vue";
import ChannelSelect from "../atoms/ChannelSelect.vue";
import RoleSelect from "../atoms/RoleSelect.vue";

const props = defineProps<{
  field: ConfigField;
  modelValue: string;
  guildId: string | null;
  modified?: boolean;
  hint?: string;
  hintSource?: string;
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
</script>

<template>
  <div class="field-row" :class="{ modified }">
    <!-- Colonne gauche : label + input + hint -->
    <div class="field-col-input">
      <label :for="field.key" class="field-label">
        {{ field.label }}
        <span v-if="modified" class="modified-dot" />
      </label>

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

      <ChannelSelect
        v-else-if="field.type === 'channel'"
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

      <span v-if="hint" class="field-hint" :class="hintSource ? `hint-${hintSource}` : ''">
        {{ hint }}
      </span>
    </div>

    <!-- Colonne droite : description pedagogique -->
    <div class="field-col-desc">
      <FieldDescription :text="field.description" />
    </div>
  </div>
</template>

<style scoped>
.field-row {
  display: grid;
  grid-template-columns: minmax(0, 3fr) minmax(0, 2fr);
  gap: 16px;
  padding: 14px 16px;
  background: var(--bg-card, #1a1d24);
  border: 1px solid var(--border);
  border-radius: 8px;
  align-items: start;
}

.field-row.modified {
  border-color: var(--accent, #5865f2);
  box-shadow: 0 0 0 1px var(--accent, #5865f2);
}

.field-col-input {
  display: flex;
  flex-direction: column;
  gap: 6px;
  min-width: 0;
}

.field-col-desc {
  display: flex;
  align-items: center;
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
}

.modified-dot {
  width: 6px;
  height: 6px;
  border-radius: 50%;
  background: var(--accent, #5865f2);
}

.field-input {
  width: 100%;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-input, #1e2128);
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

.field-hint {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: monospace;
}

.field-hint.hint-db {
  color: var(--success, #57f287);
}

.field-hint.hint-default {
  color: var(--text-secondary);
}

.field-hint.hint-none {
  color: var(--text-secondary);
  opacity: 0.6;
}

@media (max-width: 1100px) {
  .field-row {
    grid-template-columns: 1fr;
  }

  .field-col-desc {
    border-top: 1px dashed var(--border);
    padding-top: 10px;
  }
}
</style>
