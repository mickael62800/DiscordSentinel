<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  /** ISO date string si le membre est parti, null/undefined sinon. */
  leftAt?: string | null;
  /** Affichage compact (icone seule) vs avec texte. */
  compact?: boolean;
}>();

const isLeft = computed(() => !!props.leftAt);

const tooltip = computed(() => {
  if (!props.leftAt) return "";
  const d = new Date(props.leftAt);
  return `Membre parti le ${d.toLocaleDateString("fr-FR")} a ${d.toLocaleTimeString("fr-FR", { hour: "2-digit", minute: "2-digit" })}`;
});
</script>

<template>
  <span
    v-if="isLeft"
    :class="['member-status-badge', { 'member-status-badge--compact': compact }]"
    :title="tooltip"
  >
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2.2"
      stroke-linecap="round"
      stroke-linejoin="round"
      width="11"
      height="11"
    >
      <path d="M9 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h4" />
      <polyline points="16 17 21 12 16 7" />
      <line x1="21" y1="12" x2="9" y2="12" />
    </svg>
    <span v-if="!compact" class="member-status-badge__label">Parti</span>
  </span>
</template>

<style scoped>
.member-status-badge {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  padding: 2px 7px;
  border-radius: 4px;
  font-size: 10.5px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
  white-space: nowrap;
  background: color-mix(in srgb, #6b7280 18%, transparent);
  color: color-mix(in srgb, #9ca3af 100%, transparent);
  border: 1px solid color-mix(in srgb, #6b7280 30%, transparent);
}

.member-status-badge--compact {
  padding: 2px 4px;
}

.member-status-badge__label {
  line-height: 1;
}
</style>
