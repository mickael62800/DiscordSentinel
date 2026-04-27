<script setup lang="ts">
import { ref } from "vue";
import { useVoiceThemes } from "@/composables/useVoiceThemes";
import type { VoiceChannelTheme, CreateThemePayload } from "@/types/voice-extended";

const { themes, loading, create, update, remove } = useVoiceThemes();

const editingId = ref<string | null>(null);
const showCreateForm = ref(false);

const draft = ref<CreateThemePayload>({
  name: "",
  emoji: "",
  channel_name_template: "{username}",
  member_limit: null,
  visibility: "public",
  locked: false,
  queue_enabled: false,
  bitrate: null,
  slowmode_secs: null,
  stage_enabled: false,
  is_default: false,
  sort_order: 0,
});

const VISIBILITIES = [
  { key: "public", label: "Public" },
  { key: "private", label: "Privé" },
  { key: "muted", label: "Muet (read-only)" },
];

function resetDraft() {
  draft.value = {
    name: "",
    emoji: "",
    channel_name_template: "{username}",
    member_limit: null,
    visibility: "public",
    locked: false,
    queue_enabled: false,
    bitrate: null,
    slowmode_secs: null,
    stage_enabled: false,
    is_default: false,
    sort_order: 0,
  };
}

function startEdit(t: VoiceChannelTheme) {
  editingId.value = t.id;
  draft.value = {
    name: t.name,
    emoji: t.emoji,
    channel_name_template: t.channel_name_template,
    member_limit: t.member_limit,
    visibility: t.visibility,
    locked: t.locked,
    queue_enabled: t.queue_enabled,
    bitrate: t.bitrate,
    slowmode_secs: t.slowmode_secs,
    stage_enabled: t.stage_enabled,
    is_default: t.is_default,
    sort_order: t.sort_order,
  };
  showCreateForm.value = true;
}

function startCreate() {
  editingId.value = null;
  resetDraft();
  showCreateForm.value = true;
}

async function onSave() {
  if (!draft.value.name?.trim()) return;
  if (editingId.value) {
    await update(editingId.value, draft.value);
  } else {
    await create(draft.value);
  }
  showCreateForm.value = false;
  editingId.value = null;
  resetDraft();
}

function cancel() {
  showCreateForm.value = false;
  editingId.value = null;
  resetDraft();
}

async function onRemove(theme: VoiceChannelTheme) {
  if (!confirm(`Supprimer le thème "${theme.name}" ?`)) return;
  await remove(theme.id);
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>🎙️ Thèmes voice channels</h1>
      <p class="lede">
        Gabarits de salons vocaux temporaires (nom, limite, bitrate, visibilité,
        slowmode, queue, stage). Quand un membre rejoint le salon trigger
        configuré, le bot crée un salon dérivé du thème par défaut.
        Variables : <code>{username}</code>, <code>{theme}</code>.
      </p>
    </header>

    <section class="card">
      <div class="card-header">
        <h2>Thèmes existants</h2>
        <button class="btn-primary" @click="startCreate">+ Nouveau thème</button>
      </div>

      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="themes.length === 0" class="empty">
        Aucun thème — créez-en un pour permettre la création automatique de salons.
      </div>
      <table v-else class="table">
        <thead>
          <tr>
            <th></th>
            <th>Nom</th>
            <th>Visibilité</th>
            <th>Limite</th>
            <th>Bitrate</th>
            <th>Drapeaux</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="t in themes" :key="t.id">
            <td class="emoji">{{ t.emoji ?? "🎙️" }}</td>
            <td>
              <strong>{{ t.name }}</strong>
              <small class="muted">{{ t.channel_name_template }}</small>
            </td>
            <td>{{ t.visibility }}</td>
            <td>{{ t.member_limit ?? "—" }}</td>
            <td>{{ t.bitrate ? `${Math.round(t.bitrate / 1000)} kbps` : "—" }}</td>
            <td>
              <span v-if="t.is_default" class="flag default">défaut</span>
              <span v-if="t.locked" class="flag locked">verrouillé</span>
              <span v-if="t.queue_enabled" class="flag queue">queue</span>
              <span v-if="t.stage_enabled" class="flag stage">stage</span>
            </td>
            <td class="row-actions">
              <button class="btn-secondary" @click="startEdit(t)">Modifier</button>
              <button class="btn-danger" @click="onRemove(t)">🗑️</button>
            </td>
          </tr>
        </tbody>
      </table>
    </section>

    <!-- Form create/edit -->
    <div v-if="showCreateForm" class="modal-backdrop" @click.self="cancel">
      <div class="modal">
        <h3>{{ editingId ? "Modifier le thème" : "Nouveau thème" }}</h3>
        <form @submit.prevent="onSave" class="form-grid">
          <label>
            Nom *
            <input v-model="draft.name" required />
          </label>
          <label>
            Emoji
            <input v-model="draft.emoji" placeholder="🎮" />
          </label>
          <label class="full">
            Template du nom (variables {username}, {theme})
            <input v-model="draft.channel_name_template" />
          </label>
          <label>
            Visibilité
            <select v-model="draft.visibility">
              <option v-for="v in VISIBILITIES" :key="v.key" :value="v.key">{{ v.label }}</option>
            </select>
          </label>
          <label>
            Limite de membres
            <input v-model.number="draft.member_limit" type="number" min="0" placeholder="0 = illimité" />
          </label>
          <label>
            Bitrate (bps)
            <input v-model.number="draft.bitrate" type="number" placeholder="64000" />
          </label>
          <label>
            Slowmode (s)
            <input v-model.number="draft.slowmode_secs" type="number" min="0" />
          </label>
          <label>
            Sort order
            <input v-model.number="draft.sort_order" type="number" />
          </label>

          <div class="flags-row full">
            <label class="toggle">
              <input v-model="draft.locked" type="checkbox" />
              Verrouillé (admin only)
            </label>
            <label class="toggle">
              <input v-model="draft.queue_enabled" type="checkbox" />
              Queue activée
            </label>
            <label class="toggle">
              <input v-model="draft.stage_enabled" type="checkbox" />
              Stage channel
            </label>
            <label class="toggle">
              <input v-model="draft.is_default" type="checkbox" />
              Thème par défaut
            </label>
          </div>

          <div class="actions full">
            <button type="button" class="btn-secondary" @click="cancel">Annuler</button>
            <button type="submit" class="btn-primary">
              {{ editingId ? "Enregistrer" : "Créer" }}
            </button>
          </div>
        </form>
      </div>
    </div>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.card-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.card-header h2 {
  margin: 0;
}
.emoji {
  font-size: 1.4rem;
  text-align: center;
  width: 40px;
}
.flag {
  display: inline-block;
  margin-right: 4px;
  padding: 1px 6px;
  border-radius: 8px;
  font-size: 0.7rem;
  font-weight: 600;
  color: white;
}
.flag.default {
  background: #5865F2;
}
.flag.locked {
  background: #E67E22;
}
.flag.queue {
  background: #9B59B6;
}
.flag.stage {
  background: #2ECC71;
}
.row-actions {
  display: flex;
  gap: 4px;
}
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.7);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.modal {
  background: var(--bg-card, #1f1f1f);
  border-radius: 8px;
  padding: 24px;
  width: 90%;
  max-width: 700px;
  max-height: 90vh;
  overflow-y: auto;
}
.modal h3 {
  margin: 0 0 16px 0;
}
.form-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.form-grid label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.9rem;
}
.form-grid label.full {
  grid-column: span 2;
}
.form-grid input,
.form-grid select {
  background: var(--bg-input, #2a2a2a);
  border: 1px solid var(--border-color, #444);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
}
.toggle {
  flex-direction: row !important;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}
.flags-row {
  display: flex;
  gap: 16px;
  flex-wrap: wrap;
}
</style>
