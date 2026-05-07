<script setup lang="ts">
import { ref, watch } from "vue";
import type { Infraction } from "../../types";
import { conductService } from "@/services/conductService";
import AppModal from "../atoms/AppModal.vue";
import AppButton from "../atoms/AppButton.vue";

const props = defineProps<{
  visible: boolean;
  target: Infraction | null;
}>();

const emit = defineEmits<{
  close: [];
  // grant=null  → annuler sans redonner de points
  // grant=number → redonner X points avant d annuler
  confirm: [grant: number | null];
}>();

const currentPoints = ref<number | null>(null);
const loading = ref(false);
const grantAmount = ref(5);
const error = ref<string | null>(null);
const submitting = ref(false);

watch(
  () => props.visible,
  async (v) => {
    if (!v || !props.target) return;
    error.value = null;
    submitting.value = false;
    grantAmount.value = 5;
    currentPoints.value = null;
    loading.value = true;
    try {
      const points = await conductService.getPoints(
        props.target.server,
        props.target.user_id,
      );
      currentPoints.value = points.points;
    } catch (e) {
      console.warn("Echec lecture points conduite:", e);
      currentPoints.value = null;
    } finally {
      loading.value = false;
    }
  },
);

function close() {
  if (submitting.value) return;
  emit("close");
}

function confirmDeleteOnly() {
  submitting.value = true;
  emit("confirm", null);
}

function confirmWithGrant() {
  if (grantAmount.value <= 0) {
    error.value = "Le nombre de points doit etre superieur a 0.";
    return;
  }
  submitting.value = true;
  emit("confirm", grantAmount.value);
}

defineExpose({
  setError(msg: string) {
    error.value = msg;
    submitting.value = false;
  },
});
</script>

<template>
  <AppModal
    :visible="visible && !!target"
    title="Annuler la proposition de ban"
    size="lg"
    @close="close"
  >
    <template v-if="target">
      <div class="modal-user">
        <div class="avatar-placeholder user-avatar-placeholder">
          {{ target.username.charAt(0).toUpperCase() }}
        </div>
        <div class="user-info">
          <span class="username">{{ target.username }}</span>
          <span class="user-id mono">{{ target.user_id }}</span>
        </div>
        <div class="points-badge" :class="{ zero: currentPoints === 0 }">
          <span class="points-label">Points</span>
          <span class="points-value">
            {{ loading ? "…" : (currentPoints ?? "?") }}
          </span>
        </div>
      </div>

      <div class="warning">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
          <line x1="12" y1="9" x2="12" y2="13" />
          <line x1="12" y1="17" x2="12.01" y2="17" />
        </svg>
        <p>
          Si les points de conduite restent a <strong>0</strong>, le worker
          recreera automatiquement une proposition de ban dans les minutes
          qui suivent. Pour eviter cela, redonnez quelques points ci-dessous.
        </p>
      </div>

      <label class="modal-label" for="grant-input">Points a redonner</label>
      <input
        id="grant-input"
        v-model.number="grantAmount"
        type="number"
        min="1"
        max="100"
        class="modal-input"
        :disabled="submitting"
      />

      <p v-if="error" class="form-error">{{ error }}</p>
    </template>

    <template #footer>
      <AppButton variant="secondary" :disabled="submitting" @click="close">
        Fermer
      </AppButton>
      <button
        class="ghost-btn"
        :disabled="submitting"
        title="Supprimer la proposition sans modifier les points (sera probablement recreee)"
        @click="confirmDeleteOnly"
      >
        Annuler quand meme
      </button>
      <AppButton
        variant="primary"
        :disabled="submitting || grantAmount <= 0"
        @click="confirmWithGrant"
      >
        {{ submitting ? "…" : `Redonner ${grantAmount} pts et annuler` }}
      </AppButton>
    </template>
  </AppModal>
</template>

<style scoped>
.modal-user {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-lg);
  padding: var(--space-md);
  background: var(--bg-hover);
  border-radius: var(--radius-md);
}

.user-avatar-placeholder { width: 36px; height: 36px; font-size: 14px; }
.user-info { flex: 1; min-width: 0; display: flex; flex-direction: column; gap: 2px; }
.username { font-weight: 600; font-size: 14px; }
.user-id { font-size: 11px; color: var(--text-secondary); }

.points-badge {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  padding: 6px 10px;
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  border: 1px solid var(--border);
}
.points-badge.zero { border-color: var(--danger); color: var(--danger); }
.points-label { font-size: 9px; text-transform: uppercase; letter-spacing: 0.6px; color: var(--text-secondary); }
.points-value { font-size: 18px; font-weight: 700; font-family: "JetBrains Mono", monospace; }

.warning {
  display: flex;
  gap: 10px;
  padding: 12px;
  margin-bottom: var(--space-lg);
  background: color-mix(in srgb, var(--warning, #f59e0b) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--warning, #f59e0b) 35%, transparent);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-size: 13px;
  line-height: 1.4;
}
.warning svg { width: 18px; height: 18px; flex-shrink: 0; color: var(--warning, #f59e0b); margin-top: 1px; }
.warning p { margin: 0; }

.modal-label { display: block; font-size: 13px; font-weight: 600; color: var(--text-secondary); margin-bottom: var(--space-sm); }

.modal-input {
  width: 100%;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  transition: border-color var(--transition-base);
}
.modal-input:focus { border-color: var(--accent); }

.form-error { margin-top: 10px; color: var(--danger); font-size: 13px; }

.ghost-btn {
  background: transparent;
  border: 1px solid var(--danger);
  border-radius: var(--radius-sm);
  padding: var(--space-sm) var(--space-lg);
  color: var(--danger);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}
.ghost-btn:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 10%, transparent); }
.ghost-btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
