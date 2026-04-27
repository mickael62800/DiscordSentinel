<script setup lang="ts">
import { reactive, watch } from "vue";
import { useStrikes } from "@/composables/useStrikes";
import type { StrikeThreshold } from "@/types/strikes";

const {
  config,
  userStrikes,
  lookupUserId,
  loadingConfig,
  loadingStrikes,
  saving,
  saveConfig,
  lookupStrikes,
  resetStrikes,
} = useStrikes();

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

async function onSave() {
  await saveConfig({
    enabled: draft.enabled,
    window_secs: draft.window_secs,
    thresholds: draft.thresholds,
  });
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function formatDuration(secs: number | null): string {
  if (!secs) return "permanent";
  if (secs < 60) return `${secs}s`;
  if (secs < 3600) return `${Math.round(secs / 60)} min`;
  if (secs < 86400) return `${Math.round(secs / 3600)} h`;
  return `${Math.round(secs / 86400)} j`;
}
</script>

<template>
  <div class="strikes-page">
    <header class="page-header">
      <h1>⚖️ Strikes</h1>
      <p class="lede">
        Système d'escalade automatique : N strikes accumulés sur une fenêtre →
        action automatique (warn / mute / kick / ban). Les strikes expirent
        au bout de la fenêtre.
      </p>
    </header>

    <!-- ── Config ── -->
    <section class="card">
      <h2>Configuration</h2>
      <div v-if="loadingConfig" class="loading">Chargement…</div>
      <div v-else-if="!config" class="empty">
        Sélectionne une guild pour configurer.
      </div>
      <form v-else @submit.prevent="onSave" class="config-form">
        <label class="toggle">
          <input v-model="draft.enabled" type="checkbox" />
          Système de strikes actif
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
                  <input
                    v-model.number="t.duration"
                    type="number"
                    placeholder="vide = permanent"
                    :disabled="t.action === 'warn' || t.action === 'kick'"
                  />
                  <small class="muted" v-if="t.duration">{{ formatDuration(t.duration) }}</small>
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

    <!-- ── Recherche par user ── -->
    <section class="card">
      <h2>Strikes par utilisateur</h2>
      <div class="lookup">
        <input
          v-model="lookupUserId"
          placeholder="ID de l'utilisateur"
          @keyup.enter="lookupStrikes"
        />
        <button class="btn-secondary" @click="lookupStrikes">Rechercher</button>
        <button
          v-if="userStrikes.length > 0"
          class="btn-danger"
          @click="resetStrikes"
        >
          Reset tous les strikes
        </button>
      </div>

      <div v-if="loadingStrikes" class="loading">Chargement…</div>
      <div v-else-if="userStrikes.length === 0 && lookupUserId" class="empty">
        Aucun strike actif pour cet utilisateur.
      </div>
      <table v-else-if="userStrikes.length > 0" class="strikes-table">
        <thead>
          <tr>
            <th>Date</th>
            <th>Raison</th>
            <th>Source</th>
            <th>Expire</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="s in userStrikes" :key="s.id">
            <td>{{ formatDate(s.created_at) }}</td>
            <td class="reason">{{ s.reason }}</td>
            <td><code>{{ s.source }}</code></td>
            <td>{{ s.expires_at ? formatDate(s.expires_at) : "—" }}</td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>

<style scoped>
.strikes-page {
  max-width: 1100px;
  margin: 0 auto;
  padding: 24px;
}
.page-header {
  margin-bottom: 24px;
}
.page-header h1 {
  margin: 0 0 8px 0;
  font-size: 1.6rem;
}
.lede {
  color: var(--text-muted, #888);
  margin: 0;
}
.card {
  background: var(--bg-card, #1f1f1f);
  border: 1px solid var(--border-color, #333);
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 20px;
}
.card h2 {
  margin: 0 0 12px 0;
}
.config-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}
.toggle {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}
label.full {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
label.full input,
.lookup input {
  background: var(--bg-input, #2a2a2a);
  border: 1px solid var(--border-color, #444);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
  width: 100%;
  max-width: 280px;
}
.muted {
  color: var(--text-muted, #888);
  font-size: 0.85rem;
}
.thresholds h3 {
  margin: 0 0 4px 0;
  font-size: 1rem;
}
.hint {
  color: var(--text-muted, #888);
  font-size: 0.85rem;
  margin: 0 0 8px 0;
}
.thresholds-table,
.strikes-table {
  width: 100%;
  border-collapse: collapse;
  margin-bottom: 12px;
}
.thresholds-table th,
.thresholds-table td,
.strikes-table th,
.strikes-table td {
  text-align: left;
  padding: 6px 10px;
  border-bottom: 1px solid var(--border-color, #333);
  vertical-align: middle;
}
.thresholds-table th,
.strikes-table th {
  font-size: 0.85rem;
  color: var(--text-muted, #888);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.thresholds-table input,
.thresholds-table select {
  background: var(--bg-input, #2a2a2a);
  border: 1px solid var(--border-color, #444);
  border-radius: 4px;
  padding: 4px 8px;
  color: inherit;
  width: 90%;
}
.btn-icon {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 1.1rem;
}
.btn-secondary,
.btn-primary,
.btn-danger {
  border: none;
  border-radius: 4px;
  padding: 8px 18px;
  cursor: pointer;
  font-weight: 600;
}
.btn-secondary {
  background: var(--bg-input, #2a2a2a);
  color: inherit;
}
.btn-primary {
  background: #5865F2;
  color: white;
}
.btn-danger {
  background: #E74C3C;
  color: white;
}
.btn-primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
.actions {
  display: flex;
  justify-content: flex-end;
}
.lookup {
  display: flex;
  gap: 8px;
  align-items: center;
  margin-bottom: 16px;
  flex-wrap: wrap;
}
.lookup input {
  flex: 1;
  min-width: 200px;
  max-width: 320px;
}
.loading,
.empty {
  padding: 16px;
  text-align: center;
  color: var(--text-muted, #888);
}
.reason {
  max-width: 480px;
  word-break: break-word;
}
</style>
