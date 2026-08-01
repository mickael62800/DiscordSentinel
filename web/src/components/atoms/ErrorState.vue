<script setup lang="ts">
defineProps<{
  message?: string;
  retryable?: boolean;
}>();

defineEmits<{
  retry: [];
}>();
</script>

<template>
  <div class="error-state">
    <div class="error-icon">!</div>
    <p class="error-message">{{ message || "Impossible de charger les donnees. Verifiez votre connexion." }}</p>
    <button v-if="retryable" class="retry-btn" @click="$emit('retry')">Reessayer</button>
  </div>
</template>

<style scoped>
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 40px 20px;
  text-align: center;
  gap: 12px;
}

.error-icon {
  width: 48px;
  height: 48px;
  border-radius: 50%;
  background: rgba(239, 68, 68, 0.15);
  color: #ef4444;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 24px;
  font-weight: 700;
}

.error-message {
  color: var(--text-secondary);
  font-size: 14px;
  max-width: 400px;
  margin: 0;
  line-height: 1.5;
}

.retry-btn {
  margin-top: 8px;
  background: var(--accent);
  color: white;
  border: none;
  border-radius: var(--radius-sm);
  padding: 8px 20px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity var(--transition-base);
}

.retry-btn:hover {
  opacity: 0.85;
}
</style>
