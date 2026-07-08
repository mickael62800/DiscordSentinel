<script setup lang="ts">
import { ref, computed, watch } from "vue";
import type { GameTemplate } from "@/services/gamePortalService";
import AppButton from "@/components/atoms/AppButton.vue";
import AppModal from "@/components/atoms/AppModal.vue";

const props = withDefaults(
  defineProps<{
    open: boolean;
    template: GameTemplate | null;
    poolRemainingMb: number;
    suggestedName: string;
    busy?: boolean;
  }>(),
  { busy: false },
);

const emit = defineEmits<{
  close: [];
  submit: [payload: { name: string; memoryMb: number }];
}>();

const STEP = 256;

const name = ref("");
const memoryMb = ref(0);

const minMb = computed(() => props.template?.min_memory_mb ?? 0);
const maxMb = computed(() => props.template?.max_memory_mb ?? 0);

// Re-initialise le formulaire a chaque ouverture / changement de template.
watch(
  () => [props.open, props.template] as const,
  ([open]) => {
    if (!open || !props.template) return;
    name.value = props.suggestedName;
    memoryMb.value = props.template.default_memory_mb;
  },
  { immediate: true },
);

function clampMemory() {
  let v = Number(memoryMb.value);
  if (Number.isNaN(v)) v = props.template?.default_memory_mb ?? minMb.value;
  v = Math.min(maxMb.value, Math.max(minMb.value, Math.round(v)));
  memoryMb.value = v;
}

function formatMb(mb: number): string {
  if (mb >= 1024 && mb % 1024 === 0) return `${mb / 1024} Go`;
  if (mb >= 1024) return `${(mb / 1024).toFixed(1)} Go`;
  return `${mb} Mo`;
}

const trimmedName = computed(() => name.value.trim());
const exceedsPool = computed(() => memoryMb.value > props.poolRemainingMb);
const outOfBounds = computed(
  () => memoryMb.value < minMb.value || memoryMb.value > maxMb.value,
);
const poolPct = computed(() => {
  if (props.poolRemainingMb <= 0) return 100;
  return Math.min(100, Math.round((memoryMb.value / props.poolRemainingMb) * 100));
});

const canSubmit = computed(
  () =>
    !!trimmedName.value &&
    !outOfBounds.value &&
    !exceedsPool.value &&
    !props.busy,
);

function submit() {
  clampMemory();
  if (!canSubmit.value) return;
  emit("submit", { name: trimmedName.value, memoryMb: memoryMb.value });
}
</script>

<template>
  <AppModal
    :visible="open"
    :title="template ? `Nouveau serveur ${template.name}` : 'Nouveau serveur'"
    size="md"
    @close="emit('close')"
  >
    <template #header>
      <h3 class="head-title">
        <span v-if="template?.icon" class="head-icon">{{ template.icon }}</span>
        Nouveau serveur {{ template?.name ?? "" }}
      </h3>
    </template>

    <label class="field">
      <span>Nom du serveur</span>
      <input
        v-model="name"
        type="text"
        maxlength="64"
        placeholder="Ex : Survie-1"
        class="input"
        @keyup.enter="submit"
      />
    </label>

    <div class="field">
      <div class="ram-head">
        <span>RAM allouée</span>
        <div class="ram-value">
          <input
            v-model.number="memoryMb"
            type="number"
            :min="minMb"
            :max="maxMb"
            :step="STEP"
            class="input input--num"
            @change="clampMemory"
          />
          <span class="ram-unit">Mo · {{ formatMb(memoryMb) }}</span>
        </div>
      </div>
      <input
        v-model.number="memoryMb"
        type="range"
        :min="minMb"
        :max="maxMb"
        :step="STEP"
        class="slider"
        @input="clampMemory"
      />
      <div class="ram-bounds">
        <span>{{ formatMb(minMb) }}</span>
        <span>{{ formatMb(maxMb) }}</span>
      </div>
    </div>

    <div class="pool">
      <div class="pool-row">
        <span>RAM restante du pool</span>
        <strong :class="{ 'pool-over': exceedsPool }">{{ poolRemainingMb }} Mo</strong>
      </div>
      <div class="pool-bar">
        <div
          class="pool-fill"
          :class="{ 'pool-fill--over': exceedsPool }"
          :style="{ width: `${poolPct}%` }"
        />
      </div>
    </div>

    <p v-if="exceedsPool" class="warn-msg">
      Dépasse la RAM disponible ({{ poolRemainingMb }} Mo restants) — la création
      sera refusée par le serveur.
    </p>

    <template #footer>
      <AppButton variant="secondary" :disabled="busy" @click="emit('close')">
        Annuler
      </AppButton>
      <AppButton variant="primary" :disabled="!canSubmit" @click="submit">
        {{ busy ? "..." : "Créer" }}
      </AppButton>
    </template>
  </AppModal>
</template>

<style scoped>
.head-title {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}
.head-icon { font-size: 20px; line-height: 1; }

.field { display: flex; flex-direction: column; gap: 8px; margin-bottom: 18px; }
.field > span { font-size: 13px; font-weight: 500; color: var(--text-primary); }

.input {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
}
.input--num { width: 90px; text-align: right; }

.ram-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-md);
}
.ram-head > span { font-size: 13px; font-weight: 500; color: var(--text-primary); }
.ram-value { display: flex; align-items: center; gap: 8px; }
.ram-unit { font-size: 12px; color: var(--text-secondary); white-space: nowrap; }

.slider {
  width: 100%;
  accent-color: var(--accent);
  cursor: pointer;
}

.ram-bounds {
  display: flex;
  justify-content: space-between;
  font-size: 11px;
  color: var(--text-secondary);
}

.pool {
  display: flex;
  flex-direction: column;
  gap: 8px;
  padding: 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-secondary);
  margin-bottom: 14px;
}
.pool-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  font-size: 13px;
  color: var(--text-secondary);
}
.pool-row strong { color: var(--text-primary); }
.pool-over { color: var(--danger); }

.pool-bar {
  height: 8px;
  border-radius: 999px;
  background: var(--bg-primary);
  overflow: hidden;
}
.pool-fill {
  height: 100%;
  background: var(--accent);
  border-radius: 999px;
  transition: width 0.15s ease;
}
.pool-fill--over { background: var(--danger); }

.warn-msg {
  padding: 10px;
  border-radius: 6px;
  background: var(--danger-bg, rgba(239, 68, 68, 0.15));
  color: var(--danger);
  font-size: 13px;
  margin: 0 0 14px;
}
</style>
