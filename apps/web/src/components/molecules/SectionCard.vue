<script setup lang="ts">
import SectionIcon from "../atoms/SectionIcon.vue";

const props = defineProps<{
  path: string;
  label: string;
  icon: string;
  sectionKey: string;
}>();

// Le theme est derive du prefixe de la cle (ex: "moderation.strikes" -> "moderation").
const theme = props.sectionKey.split(".")[0] || "default";
</script>

<template>
  <router-link :to="path" :class="['section-card', `theme-${theme}`]" :data-section-key="sectionKey">
    <div class="icon-wrap">
      <SectionIcon :name="icon" />
    </div>
    <span class="label">{{ label }}</span>
  </router-link>
</template>

<style scoped>
.section-card {
  --theme-color: var(--accent);
  --theme-bg: color-mix(in srgb, var(--theme-color) 10%, var(--bg-card));
  --theme-border: color-mix(in srgb, var(--theme-color) 35%, var(--border));

  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 8px;
  padding: 10px 8px;
  height: 100%;
  min-height: 70px;
  border-radius: 10px;
  background-color: var(--theme-bg);
  border: 1px solid var(--theme-border);
  color: var(--text-secondary);
  text-decoration: none;
  text-align: center;
  cursor: pointer;
  transition: transform var(--transition-fast),
    border-color var(--transition-fast),
    color var(--transition-fast),
    background-color var(--transition-fast),
    box-shadow var(--transition-fast);
}

.section-card:hover {
  transform: translateY(-2px);
  border-color: var(--theme-color);
  color: var(--text-primary);
  background-color: color-mix(in srgb, var(--theme-color) 18%, var(--bg-card));
  box-shadow: 0 4px 14px color-mix(in srgb, var(--theme-color) 25%, transparent);
}

.icon-wrap {
  width: 36px;
  height: 36px;
  display: flex;
  align-items: center;
  justify-content: center;
  color: var(--theme-color);
}

.label {
  font-size: 13px;
  font-weight: 600;
  line-height: 1.2;
}

/* ── Couleurs par theme ─────────────────────── */
.theme-general    { --theme-color: #38bdf8; } /* sky    */
.theme-moderation { --theme-color: #f43f5e; } /* rose   */
.theme-community  { --theme-color: #22c55e; } /* green  */
.theme-security   { --theme-color: #f59e0b; } /* amber  */
.theme-logs       { --theme-color: #a855f7; } /* purple */
.theme-games      { --theme-color: #ec4899; } /* pink   */
.theme-config     { --theme-color: #64748b; } /* slate  */
</style>
