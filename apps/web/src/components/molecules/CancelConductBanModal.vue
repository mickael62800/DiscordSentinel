<script setup lang="ts">
import { ref, watch } from "vue";
import type { Infraction } from "../../types";
import { conductService } from "@/services/conductService";

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
  <teleport to="body">
    <div v-if="visible && target" class="modal-overlay" @click.self="close">
      <div class="card modal-content">
        <div class="modal-header">
          <h3>Annuler la proposition de ban</h3>
          <button class="modal-close" @click="close">&times;</button>
        </div>

        <div class="modal-body">
          <div class="modal-user">
            <div class="avatar-placeholder user-avatar-placeholder">
              {{ target.username.charAt(0).toUpperCase() }}
            </div>
            <div class="user-info">
              <span class="username">{{ target.username }}</span>
              <span class="user-id">{{ target.user_id }}</span>
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
        </div>

        <div class="modal-footer">
          <button
            class="modal-cancel"
            :disabled="submitting"
            @click="close"
          >
            Fermer
          </button>
          <button
            class="ghost-btn"
            :disabled="submitting"
            title="Supprimer la proposition sans modifier les points (sera probablement recreee)"
            @click="confirmDeleteOnly"
          >
            Annuler quand meme
          </button>
          <button
            class="primary-btn"
            :disabled="submitting || grantAmount <= 0"
            @click="confirmWithGrant"
          >
            {{ submitting ? "…" : `Redonner ${grantAmount} pts et annuler` }}
          </button>
        </div>
      </div>
    </div>
  </teleport>
</template>

<style scoped>
.modal-content {
  padding: 0;
  width: 100%;
  max-width: 520px;
  box-shadow: var(--shadow-lg);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: var(--space-lg) var(--space-xl);
  border-bottom: 1px solid var(--border);
}

.modal-header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.modal-close {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 24px;
  cursor: pointer;
  line-height: 1;
}

.modal-close:hover {
  color: var(--text-primary);
}

.modal-body {
  padding: var(--space-xl);
}

.modal-user {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-lg);
  padding: var(--space-md);
  background: var(--bg-hover);
  border-radius: var(--radius-md);
}

.user-avatar-placeholder {
  width: 36px;
  height: 36px;
  font-size: 14px;
}

.user-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.username {
  font-weight: 600;
  font-size: 14px;
}

.user-id {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

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

.points-badge.zero {
  border-color: var(--danger);
  color: var(--danger);
}

.points-label {
  font-size: 9px;
  text-transform: uppercase;
  letter-spacing: 0.6px;
  color: var(--text-secondary);
}

.points-value {
  font-size: 18px;
  font-weight: 700;
  font-family: "JetBrains Mono", monospace;
}

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

.warning svg {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  color: var(--warning, #f59e0b);
  margin-top: 1px;
}

.warning p {
  margin: 0;
}

.modal-label {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: var(--space-sm);
}

.modal-input {
  width: 100%;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 10px var(--space-md);
  color: var(--text-primary);
  font-size: 14px;
  font-family: inherit;
  outline: none;
  transition: border-color var(--transition-base);
}

.modal-input:focus {
  border-color: var(--accent);
}

.form-error {
  margin-top: 10px;
  color: var(--danger);
  font-size: 13px;
}

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-sm);
  padding: var(--space-lg) var(--space-xl);
  border-top: 1px solid var(--border);
  flex-wrap: wrap;
}

.modal-cancel {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-sm) var(--space-lg);
  color: var(--text-primary);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
}

.modal-cancel:hover:not(:disabled) {
  background: var(--bg-hover);
}

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

.ghost-btn:hover:not(:disabled) {
  background: color-mix(in srgb, var(--danger) 10%, transparent);
}

.primary-btn {
  background: var(--accent);
  border: none;
  border-radius: var(--radius-sm);
  padding: var(--space-sm) var(--space-lg);
  color: white;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}

.primary-btn:hover:not(:disabled) {
  opacity: 0.92;
}

.primary-btn:disabled,
.ghost-btn:disabled,
.modal-cancel:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
