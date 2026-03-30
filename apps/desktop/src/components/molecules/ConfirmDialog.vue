<script setup lang="ts">
import { useConfirm } from "../../composables/useConfirm";
import AppButton from "../atoms/AppButton.vue";

const { visible, title, message, resolve } = useConfirm();
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="confirm-overlay" @click.self="resolve(false)">
      <div class="confirm-dialog">
        <h3>{{ title }}</h3>
        <p>{{ message }}</p>
        <div class="confirm-actions">
          <AppButton variant="secondary" @click="resolve(false)">Annuler</AppButton>
          <AppButton variant="primary" @click="resolve(true)">Confirmer</AppButton>
        </div>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.confirm-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 9999;
}

.confirm-dialog {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 24px;
  min-width: 360px;
  max-width: 480px;
}

.confirm-dialog h3 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 12px;
}

.confirm-dialog p {
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 1.5;
  margin-bottom: 20px;
  white-space: pre-line;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
}
</style>
