<script setup lang="ts">
import { computed } from "vue";
import type { SystemInfo } from "@/services/systemService";

const props = defineProps<{ info: SystemInfo }>();

function formatUptime(seconds: number): string {
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}j ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

const onlineBots = computed(() => props.info.bots.filter((b) => b.online).length);
const totalBots = computed(() => props.info.bots.length);
const onlineWorkers = computed(() => props.info.workers.filter((w) => w.online).length);
const totalWorkers = computed(() => props.info.workers.length);
const allHealthy = computed(() => {
  const h = props.info.health;
  return h?.api_responding && h?.postgres_responding && h?.redis_responding;
});
</script>

<template>
  <section class="health-row">
    <div class="health-card" :class="{ ok: info.health.api_responding, ko: !info.health.api_responding }">
      <span class="health-icon">{{ info.health.api_responding ? '✅' : '❌' }}</span>
      <div class="health-text">
        <strong>API HTTP</strong>
        <span class="health-sub">{{ info.health.api_responding ? "Répond" : "Injoignable" }}</span>
      </div>
    </div>
    <div class="health-card" :class="{ ok: info.health.postgres_responding, ko: !info.health.postgres_responding }">
      <span class="health-icon">{{ info.health.postgres_responding ? '✅' : '❌' }}</span>
      <div class="health-text">
        <strong>PostgreSQL</strong>
        <span class="health-sub">{{ info.health.postgres_responding ? `${info.db_size_mb} MB` : "Erreur" }}</span>
      </div>
    </div>
    <div class="health-card" :class="{ ok: info.health.redis_responding, ko: !info.health.redis_responding }">
      <span class="health-icon">{{ info.health.redis_responding ? '✅' : '❌' }}</span>
      <div class="health-text">
        <strong>Redis</strong>
        <span class="health-sub">{{ info.health.redis_responding ? `${info.redis.used_memory_mb} MB` : "Erreur" }}</span>
      </div>
    </div>
    <div class="health-card" :class="{ ok: onlineBots === totalBots && totalBots > 0, ko: onlineBots < totalBots }">
      <span class="health-icon">🤖</span>
      <div class="health-text">
        <strong>Bots</strong>
        <span class="health-sub">{{ onlineBots }} / {{ totalBots }} en ligne</span>
      </div>
    </div>
    <div class="health-card" :class="{ ok: onlineWorkers === totalWorkers && totalWorkers > 0, ko: onlineWorkers < totalWorkers }">
      <span class="health-icon">⚙️</span>
      <div class="health-text">
        <strong>Workers</strong>
        <span class="health-sub">{{ onlineWorkers }} / {{ totalWorkers }} en ligne</span>
      </div>
    </div>
    <div class="health-card" :class="{ ok: allHealthy, ko: !allHealthy }">
      <span class="health-icon">⏱️</span>
      <div class="health-text">
        <strong>Uptime</strong>
        <span class="health-sub">{{ formatUptime(info.uptime_seconds) }}</span>
      </div>
    </div>
  </section>
</template>

<style scoped>
.health-row {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
  margin-bottom: 24px;
}
.health-card {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  transition: border-color 0.2s ease;
}
.health-card.ok {
  border-color: color-mix(in srgb, var(--success, var(--success)) 35%, var(--border));
  background: color-mix(in srgb, var(--success, var(--success)) 6%, var(--bg-card));
}
.health-card.ko {
  border-color: color-mix(in srgb, var(--danger) 50%, var(--border));
  background: color-mix(in srgb, var(--danger) 8%, var(--bg-card));
}
.health-icon { font-size: 20px; flex-shrink: 0; }
.health-text { display: flex; flex-direction: column; min-width: 0; }
.health-text strong { font-size: 13px; }
.health-sub { font-size: 11px; color: var(--text-secondary); }
</style>
