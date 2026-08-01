<script setup lang="ts">
import { useToast } from "../../composables/useToast";

const { toasts, remove } = useToast();

function icon(type: string): string {
  switch (type) {
    case "success": return "\u2713";
    case "error": return "!";
    case "warning": return "\u26A0";
    case "info": return "i";
    default: return "";
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="toast-container">
      <TransitionGroup name="toast">
        <div
          v-for="toast in toasts"
          :key="toast.id"
          :class="['toast', `toast--${toast.type}`]"
          @click="remove(toast.id)"
        >
          <span class="toast-icon">{{ icon(toast.type) }}</span>
          <span class="toast-message">{{ toast.message }}</span>
          <button class="toast-close" @click.stop="remove(toast.id)">&times;</button>
        </div>
      </TransitionGroup>
    </div>
  </Teleport>
</template>

<style scoped>
.toast-container {
  position: fixed;
  bottom: 20px;
  right: 20px;
  z-index: 10000;
  display: flex;
  flex-direction: column-reverse;
  gap: 8px;
  max-width: 400px;
  pointer-events: none;
}

.toast {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-radius: var(--radius-md);
  font-size: 13px;
  line-height: 1.4;
  box-shadow: var(--shadow-md);
  cursor: pointer;
  pointer-events: auto;
  backdrop-filter: blur(8px);
}

.toast--success {
  background: rgba(34, 197, 94, 0.15);
  border: 1px solid rgba(34, 197, 94, 0.4);
  color: #86efac;
}

.toast--error {
  background: rgba(239, 68, 68, 0.15);
  border: 1px solid rgba(239, 68, 68, 0.4);
  color: #fca5a5;
}

.toast--warning {
  background: rgba(245, 158, 11, 0.15);
  border: 1px solid rgba(245, 158, 11, 0.4);
  color: #fcd34d;
}

.toast--info {
  background: rgba(59, 130, 246, 0.15);
  border: 1px solid rgba(59, 130, 246, 0.4);
  color: #93c5fd;
}

.toast-icon {
  width: 22px;
  height: 22px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  flex-shrink: 0;
}

.toast--success .toast-icon { background: rgba(34, 197, 94, 0.3); }
.toast--error .toast-icon { background: rgba(239, 68, 68, 0.3); }
.toast--warning .toast-icon { background: rgba(245, 158, 11, 0.3); }
.toast--info .toast-icon { background: rgba(59, 130, 246, 0.3); }

.toast-message {
  flex: 1;
}

.toast-close {
  background: none;
  border: none;
  color: inherit;
  font-size: 18px;
  cursor: pointer;
  opacity: 0.6;
  padding: 0 2px;
  line-height: 1;
}

.toast-close:hover {
  opacity: 1;
}

/* Animations */
.toast-enter-active {
  transition: all 0.3s ease-out;
}

.toast-leave-active {
  transition: all 0.2s ease-in;
}

.toast-enter-from {
  opacity: 0;
  transform: translateX(40px);
}

.toast-leave-to {
  opacity: 0;
  transform: translateX(40px);
}
</style>
