<script setup lang="ts">
import type { ModerationRule } from "../../types";
import AppBadge from "../atoms/AppBadge.vue";
import AppToggle from "../atoms/AppToggle.vue";

defineProps<{
  rule: ModerationRule;
}>();

const emit = defineEmits<{
  toggle: [rule: ModerationRule];
  edit: [rule: ModerationRule];
}>();

function actionVariant(action: string): "danger" | "warning" | "info" | "default" {
  switch (action) {
    case "ban":
    case "lockdown":
      return "danger";
    case "mute":
    case "delete":
      return "warning";
    case "warn":
      return "info";
    default:
      return "default";
  }
}
</script>

<template>
  <div :class="['rule-card', { disabled: !rule.enabled }]">
    <div class="rule-header">
      <div class="rule-title">
        <h3>{{ rule.name }}</h3>
        <AppBadge :label="rule.action" :variant="actionVariant(rule.action)" />
      </div>
      <AppToggle :model-value="rule.enabled" @update:model-value="emit('toggle', rule)" />
    </div>
    <p class="rule-description">{{ rule.description }}</p>
    <div class="rule-footer">
      <AppBadge :label="rule.rule_type" variant="default" />
      <button class="edit-btn" @click="emit('edit', rule)">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7" />
          <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z" />
        </svg>
        Edit
      </button>
    </div>
  </div>
</template>

<style scoped>
.rule-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 20px;
  transition: opacity 0.2s;
}

.rule-card.disabled {
  opacity: 0.5;
}

.rule-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 12px;
}

.rule-title {
  display: flex;
  align-items: center;
  gap: 10px;
}

.rule-title h3 {
  font-size: 16px;
  font-weight: 600;
}

.rule-description {
  color: var(--text-secondary);
  font-size: 13px;
  margin-bottom: 12px;
}

.rule-footer {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.edit-btn {
  display: flex;
  align-items: center;
  gap: 6px;
  background: none;
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 500;
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid var(--border);
}

.edit-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
}

.edit-btn svg {
  width: 14px;
  height: 14px;
}
</style>
