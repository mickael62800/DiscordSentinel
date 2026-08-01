<script setup lang="ts">
import { computed } from "vue";
import type { SystemInfo } from "@/services/systemService";

const props = defineProps<{ info: SystemInfo }>();

const onlineBots = computed(() => props.info.bots.filter((b) => b.online).length);
const totalBots = computed(() => props.info.bots.length);
const onlineWorkers = computed(() => props.info.workers.filter((w) => w.online).length);
const totalWorkers = computed(() => props.info.workers.length);
</script>

<template>
  <section class="dash-section">
    <h2 class="section-title">Services Discord ({{ totalBots + totalWorkers }})</h2>
    <div class="services-grid">
      <div class="services-col">
        <h3>🤖 Bots ({{ onlineBots }} / {{ totalBots }})</h3>
        <div v-if="info.bots.length === 0" class="muted">Aucun bot enregistré.</div>
        <ul v-else class="services-list">
          <li v-for="b in info.bots" :key="b.name" :class="{ off: !b.online }">
            <span class="status-dot" :class="b.online ? 'on' : 'off'"></span>
            <span class="service-name">{{ b.name }}</span>
            <span class="service-state">{{ b.online ? 'online' : 'offline' }}</span>
          </li>
        </ul>
      </div>
      <div class="services-col">
        <h3>⚙️ Workers ({{ onlineWorkers }} / {{ totalWorkers }})</h3>
        <div v-if="info.workers.length === 0" class="muted">Aucun worker enregistré.</div>
        <ul v-else class="services-list">
          <li v-for="w in info.workers" :key="w.name" :class="{ off: !w.online }">
            <span class="status-dot" :class="w.online ? 'on' : 'off'"></span>
            <span class="service-name">{{ w.name }}</span>
            <span class="service-state">{{ w.online ? 'online' : 'offline' }}</span>
          </li>
        </ul>
      </div>
    </div>
  </section>
</template>

<style scoped>
.dash-section { margin-bottom: 24px; }
.section-title {
  position: relative;
  font-size: 14px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0 0 14px 0;
  padding: 0 0 8px 14px;
  border-bottom: 1px solid var(--border);
}
.section-title::before {
  content: "";
  position: absolute;
  left: 0;
  top: 2px;
  bottom: 14px;
  width: 3px;
  border-radius: var(--radius-xs);
  background: linear-gradient(to bottom, var(--accent), color-mix(in srgb, var(--accent) 50%, var(--accent-alt, #a855f7)));
}

.services-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
}
@media (max-width: 720px) {
  .services-grid { grid-template-columns: 1fr; }
}

.services-col {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 14px 16px;
}
.services-col h3 {
  margin: 0 0 10px;
  font-size: 14px;
  font-weight: 700;
}

.services-list {
  list-style: none;
  margin: 0;
  padding: 0;
}
.services-list li {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
  font-size: 13px;
}
.services-list li:last-child { border-bottom: none; }
.services-list li.off .service-name { color: var(--text-secondary); }

.status-dot {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  flex-shrink: 0;
}
.status-dot.on {
  background: var(--success, #2ecc71);
  box-shadow: 0 0 6px color-mix(in srgb, var(--success, #2ecc71) 60%, transparent);
}
.status-dot.off { background: var(--danger); }

.service-name {
  flex: 1;
  font-family: "JetBrains Mono", monospace;
  font-size: 12px;
}
.service-state {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.muted { color: var(--text-secondary); font-size: 13px; }
</style>
