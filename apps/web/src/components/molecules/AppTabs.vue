<script setup lang="ts">
export interface TabItem {
  key: string;
  label: string;
  /** Emoji ou caractere prefixe affiche avant le label. */
  icon?: string;
  disabled?: boolean;
}

const props = defineProps<{
  modelValue: string;
  tabs: TabItem[];
  /** "polished" : style avec gradient/glow (par defaut). "plain" : variante sobre. */
  variant?: "polished" | "plain";
}>();

const emit = defineEmits<{
  "update:modelValue": [key: string];
}>();

function select(t: TabItem) {
  if (t.disabled || t.key === props.modelValue) return;
  emit("update:modelValue", t.key);
}
</script>

<template>
  <div class="app-tabs" :class="`app-tabs--${variant ?? 'polished'}`">
    <button
      v-for="t in tabs"
      :key="t.key"
      type="button"
      class="app-tab"
      :class="{ active: modelValue === t.key }"
      :disabled="t.disabled"
      @click="select(t)"
    >
      <span v-if="t.icon" class="app-tab__icon">{{ t.icon }}</span>
      {{ t.label }}
    </button>
  </div>
</template>

<style scoped>
.app-tabs {
  display: flex;
  gap: 4px;
  position: relative;
}

.app-tab {
  position: relative;
  background: none;
  border: none;
  cursor: pointer;
  font-weight: 600;
  transition:
    color 0.2s ease,
    background 0.25s ease,
    box-shadow 0.25s ease;
}

.app-tab__icon {
  margin-right: 4px;
}

.app-tab:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

/* ============ Variant: polished (default) ============ */
.app-tabs--polished {
  padding: 4px;
  background-color: color-mix(in srgb, var(--bg-card) 80%, transparent);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
  border: 1px solid var(--border);
  border-radius: 12px;
  width: fit-content;
  box-shadow:
    inset 0 1px 2px rgba(0, 0, 0, 0.18),
    0 1px 0 color-mix(in srgb, white 6%, transparent);
}

.app-tabs--polished .app-tab {
  padding: 8px 22px;
  border-radius: 8px;
  color: var(--text-secondary);
  font-size: 0.9rem;
}

.app-tabs--polished .app-tab::after {
  content: "";
  position: absolute;
  left: 50%;
  bottom: 4px;
  width: 0;
  height: 2px;
  border-radius: 2px;
  background: var(--accent);
  transform: translateX(-50%);
  transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.app-tabs--polished .app-tab:hover:not(.active):not(:disabled) {
  color: var(--text-primary);
  background-color: color-mix(in srgb, var(--accent) 8%, transparent);
}

.app-tabs--polished .app-tab:hover:not(.active):not(:disabled)::after {
  width: 50%;
}

.app-tabs--polished .app-tab.active {
  color: white;
  background: linear-gradient(
    135deg,
    var(--accent),
    color-mix(in srgb, var(--accent) 75%, var(--accent-alt, #a855f7))
  );
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 35%, transparent),
    inset 0 -1px 0 color-mix(in srgb, black 15%, transparent),
    0 2px 10px color-mix(in srgb, var(--accent) 35%, transparent);
  text-shadow: 0 1px 1px rgba(0, 0, 0, 0.12);
}

.app-tabs--polished .app-tab:active:not(:disabled) {
  transform: scale(0.96);
  transition-duration: 0.08s;
}

/* ============ Variant: plain (sobre, pour onglets imbriques) ============ */
.app-tabs--plain {
  border-bottom: 1px solid var(--border);
}

.app-tabs--plain .app-tab {
  padding: var(--space-sm) var(--space-lg);
  border-radius: 0;
  color: var(--text-secondary);
  font-size: 13px;
  border-bottom: 2px solid transparent;
  margin-bottom: -1px;
}

.app-tabs--plain .app-tab:hover:not(.active):not(:disabled) {
  color: var(--text-primary);
}

.app-tabs--plain .app-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

@media (prefers-reduced-motion: reduce) {
  .app-tab,
  .app-tab::after {
    transition: none !important;
  }
}
</style>
