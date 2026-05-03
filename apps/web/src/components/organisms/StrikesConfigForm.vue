<script setup lang="ts">
import { reactive, watch } from "vue";
import { useStrikes } from "@/composables/useStrikes";
import type { StrikeThreshold } from "@/types/strikes";
import AppToggle from "@/components/atoms/AppToggle.vue";

const { config, loadingConfig, saving, saveConfig } = useStrikes();

const draft = reactive({
  enabled: true,
  window_secs: 86400,
  thresholds: [] as StrikeThreshold[],
});

watch(
  config,
  (c) => {
    if (!c) return;
    draft.enabled = c.enabled;
    draft.window_secs = c.window_secs;
    draft.thresholds = JSON.parse(JSON.stringify(c.thresholds));
  },
  { immediate: true },
);

const ACTIONS = ["warn", "mute", "kick", "ban"];

function addThreshold() {
  draft.thresholds.push({ strikes: 3, action: "mute", duration: 3600 });
}

function removeThreshold(idx: number) {
  draft.thresholds.splice(idx, 1);
}

function formatDuration(secs: number | null): string {
  if (!secs) return "permanent";
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.round(secs / 60)} min`;
  if (secs < 86400) return `${Math.round(secs / 3600)} h`;
  return `${Math.round(secs / 86400)} j`;
}

async function onSave() {
  await saveConfig({
    enabled: draft.enabled,
    window_secs: draft.window_secs,
    thresholds: draft.thresholds,
  });
}
</script>

<template>
  <section class="card">
    <h2>Configuration</h2>
    <div v-if="loadingConfig" class="loading">Chargement…</div>
    <div v-else-if="!config" class="empty">
      Sélectionne une guild pour configurer.
    </div>
    <form v-else @submit.prevent="onSave" class="config-form">
      <label class="toggle-row">
        <AppToggle v-model="draft.enabled" />
        <span>Système de strikes actif</span>
      </label>
      <label class="full">
        Fenêtre (secondes — au-delà, les strikes expirent)
        <input v-model.number="draft.window_secs" type="number" min="60" />
        <small class="muted">{{ formatDuration(draft.window_secs) }}</small>
      </label>

      <div class="thresholds">
        <h3>Seuils d'escalade</h3>
        <p class="hint">
          Quand le compteur de strikes actifs atteint le seuil, l'action
          est appliquée automatiquement.
        </p>
        <table v-if="draft.thresholds.length > 0" class="thresholds-table">
          <thead>
            <tr>
              <th>Seuil</th>
              <th>Action</th>
              <th>Durée (s)</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(t, idx) in draft.thresholds" :key="idx">
              <td>
                <input v-model.number="t.strikes" type="number" min="1" />
              </td>
              <td>
                <select v-model="t.action">
                  <option v-for="a in ACTIONS" :key="a" :value="a">{{ a }}</option>
                </select>
              </td>
              <td>
                <div class="cell-inline">
                  <input
                    v-model.number="t.duration"
                    type="number"
                    placeholder="vide = permanent"
                    :disabled="t.action === 'warn' || t.action === 'kick'"
                  />
                  <small class="muted" v-if="t.duration">{{ formatDuration(t.duration) }}</small>
                </div>
              </td>
              <td>
                <button type="button" class="btn-icon" @click="removeThreshold(idx)">🗑️</button>
              </td>
            </tr>
          </tbody>
        </table>
        <button type="button" class="btn-secondary" @click="addThreshold">+ Ajouter un seuil</button>
      </div>

      <div class="actions">
        <button type="submit" class="btn-primary" :disabled="saving">
          {{ saving ? "Enregistrement…" : "Enregistrer la config" }}
        </button>
      </div>
    </form>
  </section>
</template>

<style scoped>
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 20px;
}
.card h2 { margin: 0 0 12px 0; }
.config-form { display: flex; flex-direction: column; gap: 16px; }
.toggle-row {
  display: inline-flex; align-items: center; gap: 10px;
  cursor: pointer; font-weight: 500;
}
label.full {
  display: flex; flex-direction: column; gap: 6px;
  font-size: 13px; font-weight: 600; color: var(--text-secondary);
}
label.full input,
.thresholds-table input,
.thresholds-table select {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md, 8px);
  padding: 8px 12px;
  color: var(--text-primary);
  font-family: inherit; font-size: 13px; font-weight: 500;
  width: 100%; max-width: 280px; outline: none;
  transition: border-color .15s, box-shadow .15s;
}
label.full input:hover,
.thresholds-table input:hover,
.thresholds-table select:hover {
  border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
}
label.full input:focus,
.thresholds-table input:focus,
.thresholds-table select:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent);
}
label.full input:disabled,
.thresholds-table input:disabled,
.thresholds-table select:disabled { opacity: .5; cursor: not-allowed; }
label.full input[type="number"]::-webkit-inner-spin-button,
.thresholds-table input[type="number"]::-webkit-inner-spin-button {
  opacity: .6; cursor: pointer;
}
.muted { color: var(--text-secondary); font-size: 12px; }
.thresholds h3 { margin: 0 0 4px 0; font-size: 14px; font-weight: 700; }
.hint { color: var(--text-secondary); font-size: 12px; margin: 0 0 8px 0; }
.thresholds-table { width: 100%; border-collapse: collapse; margin-bottom: 12px; }
.thresholds-table th, .thresholds-table td {
  text-align: left; padding: 8px 10px;
  border-bottom: 1px solid var(--border); vertical-align: middle;
}
.thresholds-table th {
  font-size: 11px; color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: .6px; font-weight: 700;
}
.thresholds-table input, .thresholds-table select { max-width: 100%; padding: 6px 10px; }
.thresholds-table th:nth-child(1), .thresholds-table td:nth-child(1) { width: 22%; }
.thresholds-table th:nth-child(2), .thresholds-table td:nth-child(2) { width: 28%; }
.thresholds-table th:nth-child(3), .thresholds-table td:nth-child(3) { width: 42%; }
.thresholds-table th:nth-child(4), .thresholds-table td:nth-child(4) { width: 8%; text-align: right; }
.cell-inline { display: flex; align-items: center; gap: 10px; }
.cell-inline input { flex: 1; min-width: 0; }
.cell-inline .muted { white-space: nowrap; flex-shrink: 0; }
.btn-icon {
  width: 32px; height: 32px;
  display: inline-flex; align-items: center; justify-content: center;
  background: transparent; border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px); color: var(--text-secondary);
  cursor: pointer; font-size: 14px; transition: all .15s;
}
.btn-icon:hover {
  color: var(--danger); border-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 10%, transparent);
}
.btn-secondary, .btn-primary {
  border: 1px solid transparent; border-radius: var(--radius-md, 8px);
  padding: 8px 18px; cursor: pointer;
  font-size: 13px; font-weight: 600; transition: all .15s;
}
.btn-secondary { background: var(--bg-card); border-color: var(--border); color: var(--text-primary); }
.btn-secondary:hover:not(:disabled) {
  background: var(--bg-hover);
  border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
}
.btn-primary { background: var(--accent); color: white; }
.btn-primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--accent) 88%, white);
  box-shadow: 0 4px 14px color-mix(in srgb, var(--accent) 35%, transparent);
}
.btn-primary:disabled, .btn-secondary:disabled { opacity: .55; cursor: not-allowed; box-shadow: none; }
.actions { display: flex; justify-content: flex-end; }
.loading, .empty { padding: 16px; text-align: center; color: var(--text-secondary); }
</style>
