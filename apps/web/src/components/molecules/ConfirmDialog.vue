<script setup lang="ts">
import { useConfirm } from "../../composables/useConfirm";
import AppButton from "../atoms/AppButton.vue";

const { visible, title, message, resolve } = useConfirm();
</script>

<template>
  <Teleport to="body">
    <div v-if="visible" class="modal-overlay confirm-overlay" @click.self="resolve(false)">
      <div class="card card--lg confirm-dialog">
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
/* override du z-index utilitaire : dialog de confirmation au-dessus des autres modales */
.confirm-overlay {
  z-index: 9999;
}

.confirm-dialog {
  min-width: 360px;
  max-width: 480px;
}

.confirm-dialog h3 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: var(--space-md);
}

.confirm-dialog p {
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 1.5;
  margin-bottom: var(--space-lg);
  white-space: pre-line;
}

.confirm-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--space-sm);
}
</style>
