<script setup lang="ts">
import { computed, ref, watch } from "vue";

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

// --- Mode temps : quand l'unite native du champ est une duree (secondes ou
// minutes), on affiche un selecteur d'unite (sec / min / heure / jour /
// semaine) qui RE-EXPRIME la meme duree. La valeur STOCKEE reste dans l'unite
// native du champ (secondes pour unit="s", minutes pour unit="min") ; seul
// l'affichage change. Ca evite de saisir "604800" pour une semaine.
const TIME_UNITS: { key: string; label: string; secs: number }[] = [
  { key: "sec", label: "sec", secs: 1 },
  { key: "min", label: "min", secs: 60 },
  { key: "heure", label: "heure", secs: 3600 },
  { key: "jour", label: "jour", secs: 86400 },
  { key: "semaine", label: "semaine", secs: 604800 },
];

/** Nombre de secondes que vaut 1 unite native du champ, ou null si le champ
 *  n'est pas une duree (on garde alors le comportement classique). */
function baseSecsOf(unit?: string): number | null {
  const u = (unit ?? "").trim().toLowerCase();
  if (["s", "sec", "secs", "seconde", "secondes"].includes(u)) return 1;
  if (["min", "mins", "minute", "minutes"].includes(u)) return 60;
  return null;
}

const isTime = computed(() => baseSecsOf(props.unit) !== null);
const baseSecs = computed(() => baseSecsOf(props.unit) ?? 1);
// On n'offre que les unites >= a l'unite native (pas de "sec" pour un champ
// stocke en minutes : ca creerait des fractions non stockables).
const availableUnits = computed(() =>
  TIME_UNITS.filter((u) => u.secs >= baseSecs.value),
);

const numericValue = computed(() => {
  const v = props.modelValue;
  if (v === null || v === undefined || String(v).trim() === "") return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
});

/** Valeur stockee convertie en secondes (source de verite pour l'affichage). */
const storedSecs = computed(() => {
  const n = numericValue.value;
  return n === null ? null : n * baseSecs.value;
});

// Unite d'affichage courante. Auto-choisie (plus grande unite qui tombe juste)
// tant que l'utilisateur n'a pas choisi manuellement.
const unitKey = ref<string>("");
const userPicked = ref(false);

function autoPick(secs: number): string {
  const candidates = [...availableUnits.value].sort((a, b) => b.secs - a.secs);
  for (const u of candidates) {
    if (secs % u.secs === 0) return u.key;
  }
  return availableUnits.value[0]?.key ?? "sec";
}

watch(
  [storedSecs, isTime],
  () => {
    if (!isTime.value) return;
    if (!unitKey.value) unitKey.value = availableUnits.value[0]?.key ?? "sec";
    if (userPicked.value) return;
    const s = storedSecs.value;
    if (s === null || s === 0) return;
    unitKey.value = autoPick(s);
  },
  { immediate: true },
);

const unitFactor = computed(
  () => TIME_UNITS.find((u) => u.key === unitKey.value)?.secs ?? baseSecs.value,
);

/** Valeur affichee dans l'input (dans l'unite selectionnee). */
const displayValue = computed(() => {
  const s = storedSecs.value;
  if (s === null) return "";
  return String(Number((s / unitFactor.value).toFixed(4)));
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

// Pas d'increment : dans l'unite native. En mode temps, +1 dans l'unite
// affichee = `unitFactor / baseSecs` unites natives (ex. +1 heure = +60 min).
const stepValue = computed(() =>
  isTime.value ? unitFactor.value / baseSecs.value : (props.step ?? 1),
);

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

// Mode temps : saisie exprimee dans l'unite affichee -> reconvertie vers
// l'unite native stockee (entier).
function onTimeInput(e: Event) {
  const raw = (e.target as HTMLInputElement).value;
  if (raw.trim() === "") {
    emit("update:modelValue", "");
    return;
  }
  const d = Number(raw);
  if (!Number.isFinite(d)) return;
  const native = (d * unitFactor.value) / baseSecs.value;
  emit("update:modelValue", String(clamp(Math.round(native))));
}

function onUnitChange(e: Event) {
  // Changer d'unite ne modifie PAS la duree stockee : on la ré-exprime juste.
  userPicked.value = true;
  unitKey.value = (e.target as HTMLSelectElement).value;
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
        v-if="isTime"
        :id="id"
        :value="displayValue"
        type="number"
        step="any"
        min="0"
        :placeholder="placeholder"
        :required="required"
        :disabled="disabled"
        class="num-input"
        @input="onTimeInput"
      />
      <input
        v-else
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

      <select
        v-if="isTime"
        class="num-unit-select"
        :disabled="disabled"
        :value="unitKey"
        aria-label="Unité de durée"
        @change="onUnitChange"
      >
        <option v-for="u in availableUnits" :key="u.key" :value="u.key">
          {{ u.label }}
        </option>
      </select>
      <span v-else-if="unit" class="num-unit">{{ unit }}</span>

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

.num-unit-select {
  border: none;
  border-left: 1px solid var(--border);
  background: var(--bg-hover, rgba(255, 255, 255, 0.04));
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 600;
  font-family: inherit;
  letter-spacing: 0.02em;
  padding: 0 8px;
  cursor: pointer;
  outline: none;
  white-space: nowrap;
}

.num-unit-select:hover:not(:disabled) {
  color: var(--text-primary);
}

.num-unit-select:disabled {
  cursor: not-allowed;
}

.num-warn {
  font-size: 11px;
  color: var(--danger, #ef4444);
}
</style>
