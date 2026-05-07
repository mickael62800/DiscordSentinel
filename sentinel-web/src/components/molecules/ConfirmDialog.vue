<script setup lang="ts">
import { useConfirm } from "../../composables/useConfirm";
import AppButton from "../atoms/AppButton.vue";
import AppModal from "../atoms/AppModal.vue";

const { visible, title, message, resolve } = useConfirm();
</script>

<template>
  <AppModal
    :visible="visible"
    :title="title"
    size="md"
    elevated
    @close="resolve(false)"
  >
    <p class="confirm-message">{{ message }}</p>

    <template #footer>
      <AppButton variant="secondary" @click="resolve(false)">Annuler</AppButton>
      <AppButton variant="primary" @click="resolve(true)">Confirmer</AppButton>
    </template>
  </AppModal>
</template>

<style scoped>
.confirm-message {
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 1.5;
  white-space: pre-line;
  margin: 0;
}
</style>
