<script setup lang="ts">
import { useAiDataset } from "@/composables/useAiDataset";

const { items, total, counts, exporting, markAllVisible, exportAndClean } = useAiDataset();
</script>

<template>
  <section class="card stats-bar">
    <div class="stat"><span class="lbl">Affichés</span><span class="val">{{ items.length }} / {{ total }}</span></div>
    <div class="stat safe"><span class="lbl">✅ Safe</span><span class="val">{{ counts.safe }}</span></div>
    <div class="stat severe"><span class="lbl">⚠️ Severe</span><span class="val">{{ counts.severe }}</span></div>
    <div class="stat"><span class="lbl">↩ Skip</span><span class="val">{{ counts.skip }}</span></div>
    <div class="grow"></div>
    <button class="btn ghost" @click="markAllVisible('skip')">Tout skip (page)</button>
    <button class="btn ghost" @click="markAllVisible('safe')">Tout safe (page)</button>
    <button class="btn primary" :disabled="exporting || counts.total === 0" @click="exportAndClean">
      {{ exporting ? "Export…" : `📥 Exporter ${counts.total} & nettoyer` }}
    </button>
  </section>
</template>

<style scoped>
.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
  margin-bottom: 16px;
}
.stats-bar { display: flex; gap: 16px; align-items: center; flex-wrap: wrap; }
.stat { display: flex; flex-direction: column; }
.stat .lbl { font-size: 10px; text-transform: uppercase; color: var(--text-secondary); }
.stat .val { font-size: 20px; font-weight: 700; }
.stat.safe .val { color: var(--success, #2ecc71); }
.stat.severe .val { color: var(--danger); }
.grow { flex: 1; }
.btn {
  padding: 7px 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px; font-weight: 600; cursor: pointer;
}
.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
.btn.primary { background: var(--accent); color: white; border-color: var(--accent); }
.btn.primary:hover:not(:disabled) { filter: brightness(1.1); color: white; }
.btn.ghost { background: transparent; }
</style>
