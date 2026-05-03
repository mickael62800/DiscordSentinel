<script setup lang="ts">
import type { SystemInfo } from "@/services/systemService";

defineProps<{ info: SystemInfo }>();

function formatGb(gb: number): string {
  if (gb < 1) return `${(gb * 1024).toFixed(0)} MB`;
  return `${gb.toFixed(1)} GB`;
}

function diskBarColor(pct: number): string {
  if (pct >= 90) return "var(--danger)";
  if (pct >= 75) return "var(--warning, #e67e22)";
  return "var(--success, #2ecc71)";
}
</script>

<template>
  <section v-if="info.disks.length > 0" class="dash-section">
    <h2 class="section-title">Disques & montages</h2>
    <div class="disk-list">
      <div v-for="d in info.disks" :key="`${d.name}-${d.mount_point}`" class="disk-card">
        <div class="disk-header">
          <span class="disk-mount">📂 {{ d.mount_point }}</span>
          <span class="disk-pct" :style="{ color: diskBarColor(d.usage_percent) }">
            {{ d.usage_percent.toFixed(1) }}%
          </span>
        </div>
        <div class="bar">
          <div
            class="bar-fill"
            :style="{ width: `${Math.min(d.usage_percent, 100)}%`, background: diskBarColor(d.usage_percent) }"
          ></div>
        </div>
        <div class="disk-meta">
          <span><strong>{{ formatGb(d.used_gb) }}</strong> utilisé</span>
          <span class="muted">{{ formatGb(d.available_gb) }} libre</span>
          <span class="muted">{{ formatGb(d.total_gb) }} total</span>
          <span class="disk-fs">{{ d.fs_type }}</span>
          <span v-if="d.name" class="disk-name">{{ d.name }}</span>
        </div>
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
  border-radius: 2px;
  background: linear-gradient(to bottom, var(--accent), color-mix(in srgb, var(--accent) 50%, var(--accent-alt, #a855f7)));
}

.disk-list {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
  gap: 14px;
}
.disk-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px;
}
.disk-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 8px;
}
.disk-mount {
  font-family: "JetBrains Mono", monospace;
  font-size: 13px;
  font-weight: 600;
}
.disk-pct {
  font-size: 16px;
  font-weight: 700;
}
.disk-meta {
  display: flex;
  gap: 12px;
  flex-wrap: wrap;
  margin-top: 8px;
  font-size: 11px;
}
.disk-meta .muted { color: var(--text-secondary); }
.disk-fs, .disk-name {
  background: var(--bg-secondary);
  padding: 1px 6px;
  border-radius: 4px;
  font-family: "JetBrains Mono", monospace;
  color: var(--text-secondary);
}

.bar {
  height: 8px;
  background: var(--bg-secondary);
  border-radius: 4px;
  overflow: hidden;
}
.bar-fill {
  height: 100%;
  border-radius: 4px;
  transition: width 0.4s ease, background 0.3s ease;
}
</style>
