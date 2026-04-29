<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  modelValue: string;
  id?: string;
  unit?: string;
  min?: number;
  max?: number;
  placeholder?: string;
}>();

const emit = defineEmits<{
  (e: "update:modelValue", value: string): void;
}>();

const numericValue = computed(() => {
  const n = Number(props.modelValue);
  return Number.isFinite(n) ? n : null;
});

const outOfRange = computed(() => {
  const n = numericValue.value;
  if (n === null) return false;
  if (props.min !== undefined && n < props.min) return true;
  if (props.max !== undefined && n > props.max) return true;
  return false;
});

const rangeMessage = computed(() => {
  if (!outOfRange.value) return "";
  const parts: string[] = [];
  if (props.min !== undefined) parts.push(`min ${props.min}`);
  if (props.max !== undefined) parts.push(`max ${props.max}`);
  return `Hors borne (${parts.join(" – ")})`;
});

function onInput(e: Event) {
  emit("update:modelValue", (e.target as HTMLInputElement).value);
}
</script>

<template>
  <div class="num-input-wrap">
    <div class="num-input-row" :class="{ 'out-of-range': outOfRange }">
      <input
        :id="id"
        :value="modelValue"
        type="number"
        :min="min"
        :max="max"
        :placeholder="placeholder"
        class="num-input"
        @input="onInput"
      />
      <span v-if="unit" class="num-unit">{{ unit }}</span>
    </div>
    <span v-if="outOfRange" class="num-warn">{{ rangeMessage }}</span>
  </div>
</template>

<style scoped>
.num-input-wrap {
  display: flex;
  flex-direction: column;
  gap: 4px;
  width: 100%;
}

.num-input-row {
  display: flex;
  align-items: stretch;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-card);
  overflow: hidden;
  transition: border-color 0.15s;
}

.num-input-row:focus-within {
  border-color: var(--accent);
}

.num-input-row.out-of-range {
  border-color: var(--danger, #ef4444);
}

.num-input {
  flex: 1;
  padding: 8px 12px;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  min-width: 0;
}

.num-unit {
  display: flex;
  align-items: center;
  padding: 0 12px;
  background: var(--bg-hover, rgba(255, 255, 255, 0.04));
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  border-left: 1px solid var(--border);
  white-space: nowrap;
}

.num-warn {
  font-size: 11px;
  color: var(--danger, #ef4444);
}
</style>
