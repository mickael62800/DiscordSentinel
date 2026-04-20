<script setup lang="ts">
import { ref, watch } from "vue";
import type { Infraction } from "../../types";

const props = defineProps<{
  visible: boolean;
  target: Infraction | null;
  banning: boolean;
}>();

const emit = defineEmits<{
  close: [];
  confirm: [reason: string];
}>();

const reason = ref("");
const error = ref<string | null>(null);

watch(
  () => props.visible,
  (v) => {
    if (v && props.target) {
      reason.value = props.target.reason || "";
      error.value = null;
    }
  },
);

function close() {
  emit("close");
}

function confirm() {
  const r = reason.value.trim() || "Aucune raison specifiee";
  emit("confirm", r);
}

/** Permet au parent de signaler une erreur apres le confirm. */
function setError(msg: string) {
  error.value = msg;
}

defineExpose({ setError });
</script>

<template>
  <teleport to="body">
    <div v-if="visible && target" class="modal-overlay" @click.self="close">
      <div class="card modal-content">
        <div class="modal-header">
          <h3>Bannir un utilisateur</h3>
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
          </div>

          <label class="modal-label">Raison du bannissement</label>
          <textarea
            v-model="reason"
            class="modal-textarea"
            rows="3"
            placeholder="Indiquez la raison du bannissement..."
          ></textarea>

          <p v-if="error" class="ban-error">{{ error }}</p>
        </div>

        <div class="modal-footer">
          <button class="modal-cancel" @click="close">Annuler</button>
          <button
            class="ban-btn"
            :disabled="banning || !reason.trim()"
            @click="confirm"
          >
            {{ banning ? 'Bannissement...' : 'Confirmer le ban' }}
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
  max-width: 480px;
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

.modal-label {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: var(--space-sm);
}

.modal-textarea {
  width: 100%;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 10px var(--space-md);
  color: var(--text-primary);
  font-size: 14px;
  font-family: inherit;
  resize: vertical;
  outline: none;
  transition: border-color var(--transition-base);
}

.modal-textarea:focus {
  border-color: var(--accent);
}

.modal-textarea::placeholder {
  color: var(--text-secondary);
  opacity: 0.6;
}

.ban-error {
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
  transition: background var(--transition-base);
}

.modal-cancel:hover {
  background: var(--bg-hover);
}

.ban-btn {
  background: var(--danger);
  color: white;
  border: none;
  border-radius: var(--radius-sm);
  padding: var(--space-sm) var(--space-lg);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity var(--transition-base);
}

.ban-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.ban-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
</style>
