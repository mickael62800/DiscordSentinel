<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  modelValue: string | number | null | undefined;
  id?: string;
  unit?: string;
  min?: number;
  max?: number;
  step?: number;
  placeholder?: string;
  required?: boolean;
  disabled?: boolean;
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

const stepValue = computed(() => props.step ?? 1);

const canDecrement = computed(() => {
  const n = numericValue.value;
  if (n === null) return true;
  if (props.min === undefined) return true;
  return n - stepValue.value >= props.min;
});

const canIncrement = computed(() => {
  const n = numericValue.value;
  if (n === null) return true;
  if (props.max === undefined) return true;
  return n + stepValue.value <= props.max;
});

function clamp(n: number): number {
  if (props.min !== undefined && n < props.min) return props.min;
  if (props.max !== undefined && n > props.max) return props.max;
  return n;
}

function decrement() {
  const n = numericValue.value ?? 0;
  emit("update:modelValue", String(clamp(n - stepValue.value)));
}

function increment() {
  const n = numericValue.value ?? 0;
  emit("update:modelValue", String(clamp(n + stepValue.value)));
}

function onInput(e: Event) {
  emit("update:modelValue", (e.target as HTMLInputElement).value);
}
</script>

<template>
  <div class="num-input-wrap">
    <div class="num-input-row" :class="{ 'out-of-range': outOfRange, 'is-disabled': disabled }">
      <button
        type="button"
        class="num-btn num-btn-minus"
        :disabled="disabled || !canDecrement"
        tabindex="-1"
        aria-label="Diminuer"
        @click="decrement"
      >
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
          <path d="M2 6h8" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
        </svg>
      </button>

      <input
        :id="id"
        :value="modelValue ?? ''"
        type="number"
        :min="min"
        :max="max"
        :step="step"
        :placeholder="placeholder"
        :required="required"
        :disabled="disabled"
        class="num-input"
        @input="onInput"
      />

      <span v-if="unit" class="num-unit">{{ unit }}</span>

      <button
        type="button"
        class="num-btn num-btn-plus"
        :disabled="disabled || !canIncrement"
        tabindex="-1"
        aria-label="Augmenter"
        @click="increment"
      >
        <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true">
          <path
            d="M6 2v8M2 6h8"
            stroke="currentColor"
            stroke-width="2"
            stroke-linecap="round"
          />
        </svg>
      </button>
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
  transition: border-color 0.15s, box-shadow 0.15s;
}

.num-input-row:focus-within {
  border-color: var(--accent);
  box-shadow: 0 0 0 2px rgba(88, 101, 242, 0.15);
}

.num-input-row.out-of-range {
  border-color: var(--danger, #ef4444);
}

.num-input-row.is-disabled {
  opacity: 0.55;
  cursor: not-allowed;
}

.num-input {
  flex: 1;
  padding: 8px 10px;
  border: none;
  background: transparent;
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  font-variant-numeric: tabular-nums;
  text-align: center;
  outline: none;
  min-width: 0;
  -moz-appearance: textfield;
}

.num-input::-webkit-outer-spin-button,
.num-input::-webkit-inner-spin-button {
  -webkit-appearance: none;
  appearance: none;
  margin: 0;
}

.num-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  padding: 0;
  border: none;
  background: var(--bg-hover, rgba(255, 255, 255, 0.04));
  color: var(--text-secondary);
  cursor: pointer;
  font-family: inherit;
  transition: background 0.12s, color 0.12s, transform 0.08s;
  user-select: none;
}

.num-btn:hover:not(:disabled) {
  background: var(--accent);
  color: #fff;
}

.num-btn:active:not(:disabled) {
  transform: scale(0.92);
}

.num-btn:disabled {
  opacity: 0.35;
  cursor: not-allowed;
}

.num-btn-minus {
  border-right: 1px solid var(--border);
}

.num-btn-plus {
  border-left: 1px solid var(--border);
}

.num-unit {
  display: flex;
  align-items: center;
  padding: 0 10px;
  background: var(--bg-hover, rgba(255, 255, 255, 0.04));
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  border-left: 1px solid var(--border);
  white-space: nowrap;
}

.num-warn {
  font-size: 11px;
  color: var(--danger, #ef4444);
}
</style>
