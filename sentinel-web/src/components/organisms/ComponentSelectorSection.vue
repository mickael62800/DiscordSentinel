<script setup lang="ts">
import type { BotDefinition } from "../../types";
import { useBotEnabledStatus } from "@/composables/useBotEnabledStatus";

defineProps<{
  title: string;
  definitions: BotDefinition[];
  selectedKey: string | null;
}>();

const emit = defineEmits<{
  (e: "select", name: string): void;
}>();

// On utilise enabledMap (ref reactif) directement plutot que la
// fonction isBotEnabled, sinon Vue ne re-render pas le badge OFF
// apres invalidation du store (la closure casse le tracking).
const { enabledMap } = useBotEnabledStatus();

function isOn(botName: string): boolean {
  const v = enabledMap.value[botName];
  return v === undefined ? true : v;
}
</script>

<template>
  <section class="component-section">
    <div class="section-header">
      <h2 class="section-heading">{{ title }}</h2>
    </div>
    <div class="component-grid">
      <div
        v-for="def in definitions"
        :key="def.bot_name"
        class="component-card"
        :class="{
          active: selectedKey === def.bot_name,
          'is-disabled': !isOn(def.bot_name),
        }"
        @click="emit('select', def.bot_name)"
      >
        <div class="component-card-header">
          <div class="component-name">{{ def.display_name }}</div>
          <span v-if="!isOn(def.bot_name)" class="off-pill" title="Désactivé pour cette guild">OFF</span>
        </div>
        <div class="component-desc">{{ def.description }}</div>
        <div class="component-params">
          {{ def.config_schema.length }} parametre{{ def.config_schema.length > 1 ? "s" : "" }}
        </div>
      </div>
    </div>
  </section>
</template>

<style scoped>
.component-section { margin-bottom: 24px; }

.section-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border);
}

.section-heading {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0;
}

.section-count {
  font-size: 11px;
  font-weight: 600;
  color: var(--accent);
  background: rgba(99, 102, 241, 0.12);
  padding: 2px 8px;
  border-radius: 10px;
}

.component-grid {
  display: grid;
  grid-template-columns: repeat(5, 1fr);
  gap: 12px;
}

.component-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 16px;
  cursor: pointer;
  transition: border-color var(--transition-fast);
}

.component-card:hover { border-color: var(--accent); }
.component-card.active {
  border-color: var(--accent);
  background: rgba(99, 102, 241, 0.08);
}

/* Composant desactive pour cette guild : bordure rouge + tint subtil. */
.component-card.is-disabled {
  border-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 6%, var(--bg-secondary));
}
.component-card.is-disabled:hover {
  border-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 10%, var(--bg-secondary));
}
.component-card.is-disabled.active {
  border-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 14%, var(--bg-secondary));
}
.off-pill {
  font-size: 9px;
  font-weight: 700;
  letter-spacing: 0.6px;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--danger);
  color: white;
}

.component-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 4px;
}

.component-name {
  font-weight: 600;
  font-size: 15px;
  color: var(--text-primary);
}

.component-desc {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

.component-params {
  font-size: 11px;
  color: var(--accent);
  font-weight: 500;
}

@media (max-width: 640px) {
  .component-grid {
    grid-template-columns: 1fr;
    gap: 10px;
  }
  .component-card { padding: 12px; }
}
</style>
