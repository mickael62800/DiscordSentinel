<script setup lang="ts">
import { ref, watch } from "vue";
import type { Infraction } from "../../types";
import AppModal from "../atoms/AppModal.vue";
import AppButton from "../atoms/AppButton.vue";

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

function setError(msg: string) {
  error.value = msg;
}

defineExpose({ setError });
</script>

<template>
  <AppModal
    :visible="visible && !!target"
    title="Bannir un utilisateur"
    size="md"
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
      </div>

      <label class="modal-label">Raison du bannissement</label>
      <textarea
        v-model="reason"
        class="modal-textarea"
        rows="3"
        placeholder="Indiquez la raison du bannissement..."
      ></textarea>

      <p v-if="error" class="ban-error">{{ error }}</p>
    </template>

    <template #footer>
      <AppButton variant="secondary" @click="close">Annuler</AppButton>
      <AppButton
        variant="danger"
        :disabled="banning || !reason.trim()"
        @click="confirm"
      >
        {{ banning ? 'Bannissement...' : 'Confirmer le ban' }}
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

.user-avatar-placeholder {
  width: 36px;
  height: 36px;
  font-size: 14px;
}

.user-info { display: flex; flex-direction: column; gap: 2px; }
.username { font-weight: 600; font-size: 14px; }
.user-id { font-size: 11px; color: var(--text-secondary); }

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

.modal-textarea:focus { border-color: var(--accent); }
.modal-textarea::placeholder { color: var(--text-secondary); opacity: 0.6; }

.ban-error { margin-top: 10px; color: var(--danger); font-size: 13px; }
</style>
