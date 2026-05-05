<script setup lang="ts">
withDefaults(
  defineProps<{
    title: string;
    /** Emoji ou caractere prefixe affiche avant le titre. */
    icon?: string;
    /** Largeur de page : constrained (defaut), wide pour dashboards/tables, narrow pour login. */
    width?: "constrained" | "wide" | "narrow";
  }>(),
  { width: "constrained" },
);
</script>

<template>
  <div :class="['admin-page', `page--${width}`]">
    <header class="admin-page-header" :class="{ 'has-actions': !!$slots.actions }">
      <div class="admin-page-title-block">
        <h1 class="admin-page-title">
          <span v-if="icon" class="admin-page-icon">{{ icon }}</span>
          {{ title }}
        </h1>
        <p v-if="$slots.lede" class="admin-page-lede">
          <slot name="lede" />
        </p>
      </div>
      <div v-if="$slots.actions" class="admin-page-actions">
        <slot name="actions" />
      </div>
    </header>

    <slot />
  </div>
</template>

<style scoped>
.admin-page-header {
  margin-bottom: 24px;
}
.admin-page-header.has-actions {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 16px;
}
.admin-page-title-block { flex: 1; min-width: 0; }
.admin-page-title {
  margin: 0 0 8px 0;
  font-size: 22px;
  font-weight: 700;
  display: flex;
  align-items: center;
  gap: 10px;
}
.admin-page-icon { font-size: 1.2em; flex-shrink: 0; }
.admin-page-lede {
  color: var(--text-secondary);
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
}
.admin-page-lede :deep(code) {
  background: var(--bg-card);
  border: 1px solid var(--border);
  padding: 1px 6px;
  border-radius: 6px;
  font-size: 0.9em;
  font-family: "JetBrains Mono", monospace;
  color: var(--accent);
}
.admin-page-actions {
  display: flex;
  gap: 8px;
  align-items: center;
  flex-shrink: 0;
  flex-wrap: wrap;
}

@media (max-width: 768px) {
  .admin-page-header.has-actions {
    flex-direction: column;
    align-items: stretch;
  }
  .admin-page-actions { width: 100%; }
  .admin-page-actions :deep(> *) { flex: 1; }
}
</style>
