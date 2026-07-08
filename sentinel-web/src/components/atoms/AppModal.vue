<script setup lang="ts">
import { onBeforeUnmount, watch } from "vue";

const props = withDefaults(
  defineProps<{
    visible: boolean;
    title?: string;
    /** sm=360, md=480, lg=640, xl=800 */
    size?: "sm" | "md" | "lg" | "xl";
    /** Si true, place l'overlay au-dessus des autres modales (z-index 9999). */
    elevated?: boolean;
    closeOnOverlay?: boolean;
    closeOnEsc?: boolean;
  }>(),
  {
    title: "",
    size: "md",
    elevated: false,
    closeOnOverlay: true,
    closeOnEsc: true,
  },
);

const emit = defineEmits<{ close: [] }>();

function handleOverlay() {
  if (props.closeOnOverlay) emit("close");
}

function onKey(e: KeyboardEvent) {
  if (e.key === "Escape" && props.visible && props.closeOnEsc) emit("close");
}

watch(
  () => props.visible,
  (v) => {
    if (v) document.addEventListener("keydown", onKey);
    else document.removeEventListener("keydown", onKey);
  },
);

onBeforeUnmount(() => document.removeEventListener("keydown", onKey));
</script>

<template>
  <Teleport to="body">
    <div
      v-if="visible"
      class="modal-overlay"
      :class="{ 'modal-overlay--elevated': elevated }"
      @click.self="handleOverlay"
    >
      <div class="card modal-shell" :class="`modal-shell--${size}`">
        <header v-if="title || $slots.header" class="modal-head">
          <slot name="header">
            <h3>{{ title }}</h3>
          </slot>
          <button class="modal-close" type="button" aria-label="Fermer" @click="emit('close')">
            &times;
          </button>
        </header>

        <div class="modal-body">
          <slot />
        </div>

        <footer v-if="$slots.footer" class="modal-foot">
          <slot name="footer" />
        </footer>
      </div>
    </div>
  </Teleport>
</template>

<style scoped>
.modal-overlay--elevated {
  z-index: 9999;
}

.modal-shell {
  padding: 0;
  width: 100%;
  display: flex;
  flex-direction: column;
  max-height: 90vh;
  box-shadow: var(--shadow-lg);
}

.modal-shell--sm { max-width: 360px; }
.modal-shell--md { max-width: 480px; }
.modal-shell--lg { max-width: 640px; }
.modal-shell--xl { max-width: 800px; }

.modal-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: var(--space-md);
  padding: var(--space-lg) var(--space-xl);
  border-bottom: 1px solid var(--border);
}

.modal-head :deep(h2),
.modal-head :deep(h3) {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.modal-close {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 24px;
  line-height: 1;
  cursor: pointer;
  padding: 0;
  flex-shrink: 0;
}

.modal-close:hover { color: var(--text-primary); }

.modal-body {
  padding: var(--space-xl);
  overflow-y: auto;
  flex: 1;
  min-height: 0;
}

.modal-foot {
  display: flex;
  justify-content: flex-end;
  flex-wrap: wrap;
  gap: var(--space-sm);
  padding: var(--space-lg) var(--space-xl);
  border-top: 1px solid var(--border);
}
</style>
