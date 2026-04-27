<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import { useWelcome } from "@/composables/useWelcome";

const { config, loading, saving, saveConfig } = useWelcome();

// Brouillon edite par le formulaire — synchronise avec config quand
// elle arrive du backend.
const draft = reactive({
  // Welcome
  welcome_enabled: false,
  welcome_channel_id: "",
  welcome_message: "",
  welcome_title: "",
  welcome_embed_color: "#5865F2",
  welcome_image_url: "",
  welcome_footer_text: "",
  welcome_dm_enabled: false,
  welcome_dm_message: "",
  // Leave
  leave_enabled: false,
  leave_channel_id: "",
  leave_message: "",
  leave_title: "",
  leave_image_url: "",
  leave_footer_text: "",
  // Rules
  rules_enabled: false,
  rules_channel_id: "",
  rules_message: "",
  rules_role_id: "",
  rules_button_label: "",
  // Counter
  counter_enabled: false,
  counter_channel_id: "",
  counter_format: "",
  // Anniversary
  anniversary_enabled: false,
  anniversary_channel_id: "",
  anniversary_message: "",
  anniversary_title: "",
  anniversary_image_url: "",
  anniversary_footer_text: "",
  // Rejoin
  rejoin_message: "",
  rejoin_title: "",
  rejoin_image_url: "",
  rejoin_footer_text: "",
});

watch(
  config,
  (c) => {
    if (!c) return;
    draft.welcome_enabled = c.welcome_enabled;
    draft.welcome_channel_id = c.welcome_channel_id ?? "";
    draft.welcome_message = c.welcome_message;
    draft.welcome_title = c.welcome_title;
    draft.welcome_embed_color = c.welcome_embed_color || "#5865F2";
    draft.welcome_image_url = c.welcome_image_url;
    draft.welcome_footer_text = c.welcome_footer_text;
    draft.welcome_dm_enabled = c.welcome_dm_enabled;
    draft.welcome_dm_message = c.welcome_dm_message;
    draft.leave_enabled = c.leave_enabled;
    draft.leave_channel_id = c.leave_channel_id ?? "";
    draft.leave_message = c.leave_message;
    draft.leave_title = c.leave_title;
    draft.leave_image_url = c.leave_image_url;
    draft.leave_footer_text = c.leave_footer_text;
    draft.rules_enabled = c.rules_enabled;
    draft.rules_channel_id = c.rules_channel_id ?? "";
    draft.rules_message = c.rules_message;
    draft.rules_role_id = c.rules_role_id ?? "";
    draft.rules_button_label = c.rules_button_label;
    draft.counter_enabled = c.counter_enabled;
    draft.counter_channel_id = c.counter_channel_id ?? "";
    draft.counter_format = c.counter_format;
    draft.anniversary_enabled = c.anniversary_enabled;
    draft.anniversary_channel_id = c.anniversary_channel_id ?? "";
    draft.anniversary_message = c.anniversary_message;
    draft.anniversary_title = c.anniversary_title;
    draft.anniversary_image_url = c.anniversary_image_url;
    draft.anniversary_footer_text = c.anniversary_footer_text;
    draft.rejoin_message = c.rejoin_message;
    draft.rejoin_title = c.rejoin_title;
    draft.rejoin_image_url = c.rejoin_image_url;
    draft.rejoin_footer_text = c.rejoin_footer_text;
  },
  { immediate: true },
);

const previewWelcomeText = computed(() => {
  // Substitutions simples pour preview — coté bot ce sera via tera/format.
  return draft.welcome_message
    .replace(/\{user\}/g, "@NouveauMembre")
    .replace(/\{server\}/g, "Mon Serveur")
    .replace(/\{count\}/g, "42");
});

async function onSave() {
  await saveConfig({
    welcome_enabled: draft.welcome_enabled,
    welcome_channel_id: draft.welcome_channel_id || null,
    welcome_message: draft.welcome_message,
    welcome_title: draft.welcome_title,
    welcome_embed_color: draft.welcome_embed_color,
    welcome_image_url: draft.welcome_image_url,
    welcome_footer_text: draft.welcome_footer_text,
    welcome_dm_enabled: draft.welcome_dm_enabled,
    welcome_dm_message: draft.welcome_dm_message,
    leave_enabled: draft.leave_enabled,
    leave_channel_id: draft.leave_channel_id || null,
    leave_message: draft.leave_message,
    leave_title: draft.leave_title,
    leave_image_url: draft.leave_image_url,
    leave_footer_text: draft.leave_footer_text,
    rules_enabled: draft.rules_enabled,
    rules_channel_id: draft.rules_channel_id || null,
    rules_message: draft.rules_message,
    rules_role_id: draft.rules_role_id || null,
    rules_button_label: draft.rules_button_label,
    counter_enabled: draft.counter_enabled,
    counter_channel_id: draft.counter_channel_id || null,
    counter_format: draft.counter_format,
    anniversary_enabled: draft.anniversary_enabled,
    anniversary_channel_id: draft.anniversary_channel_id || null,
    anniversary_message: draft.anniversary_message,
    anniversary_title: draft.anniversary_title,
    anniversary_image_url: draft.anniversary_image_url,
    anniversary_footer_text: draft.anniversary_footer_text,
    rejoin_message: draft.rejoin_message,
    rejoin_title: draft.rejoin_title,
    rejoin_image_url: draft.rejoin_image_url,
    rejoin_footer_text: draft.rejoin_footer_text,
  });
}
</script>

<template>
  <div class="welcome-page">
    <header class="page-header">
      <h1>👋 Welcome / Onboarding</h1>
      <p class="lede">
        Configure le message de bienvenue, le verification gate (règles à
        accepter), le compteur de membres, l'anniversaire d'arrivée, et le
        message de départ. Variables disponibles dans les messages :
        <code>{user}</code>, <code>{server}</code>, <code>{count}</code>.
      </p>
    </header>

    <div v-if="loading" class="loading">Chargement…</div>

    <div v-else-if="!config" class="empty">
      Sélectionne une guild dans le menu en haut pour configurer.
    </div>

    <form v-else class="welcome-form" @submit.prevent="onSave">
      <!-- ── Welcome ── -->
      <fieldset class="card">
        <legend>
          <label class="toggle">
            <input v-model="draft.welcome_enabled" type="checkbox" />
            Message de bienvenue actif
          </label>
        </legend>
        <div class="grid">
          <label>
            Salon
            <input v-model="draft.welcome_channel_id" placeholder="ID du salon" />
          </label>
          <label>
            Titre embed
            <input v-model="draft.welcome_title" placeholder="Bienvenue !" />
          </label>
          <label>
            Couleur (hex)
            <input v-model="draft.welcome_embed_color" type="color" />
          </label>
          <label>
            Image (URL)
            <input v-model="draft.welcome_image_url" placeholder="https://..." />
          </label>
          <label class="full">
            Message
            <textarea v-model="draft.welcome_message" rows="3"></textarea>
          </label>
          <label class="full">
            Footer
            <input v-model="draft.welcome_footer_text" />
          </label>
        </div>

        <details class="dm-details">
          <summary>DM de bienvenue (optionnel)</summary>
          <label class="toggle">
            <input v-model="draft.welcome_dm_enabled" type="checkbox" />
            Activer le DM
          </label>
          <label class="full">
            Message DM
            <textarea v-model="draft.welcome_dm_message" rows="3"></textarea>
          </label>
        </details>

        <details v-if="draft.welcome_enabled" class="preview">
          <summary>👁️ Aperçu</summary>
          <div class="preview-embed" :style="{ borderLeftColor: draft.welcome_embed_color }">
            <strong v-if="draft.welcome_title">{{ draft.welcome_title }}</strong>
            <p>{{ previewWelcomeText }}</p>
            <small v-if="draft.welcome_footer_text">{{ draft.welcome_footer_text }}</small>
          </div>
        </details>
      </fieldset>

      <!-- ── Verification gate (règles) ── -->
      <fieldset class="card">
        <legend>
          <label class="toggle">
            <input v-model="draft.rules_enabled" type="checkbox" />
            🔒 Verification gate (lecture des règles)
          </label>
        </legend>
        <p class="hint">
          Affiche un bouton « J'ai lu les règles » dans le salon dédié.
          Le rôle configuré est attribué après acceptation.
        </p>
        <div class="grid">
          <label>
            Salon des règles
            <input v-model="draft.rules_channel_id" placeholder="ID du salon" />
          </label>
          <label>
            Rôle attribué
            <input v-model="draft.rules_role_id" placeholder="ID du rôle" />
          </label>
          <label>
            Texte du bouton
            <input v-model="draft.rules_button_label" placeholder="J'ai lu les règles" />
          </label>
          <label class="full">
            Message
            <textarea v-model="draft.rules_message" rows="3"></textarea>
          </label>
        </div>
      </fieldset>

      <!-- ── Compteur de membres ── -->
      <fieldset class="card">
        <legend>
          <label class="toggle">
            <input v-model="draft.counter_enabled" type="checkbox" />
            🔢 Compteur de membres
          </label>
        </legend>
        <div class="grid">
          <label>
            Salon
            <input v-model="draft.counter_channel_id" placeholder="ID du salon" />
          </label>
          <label>
            Format
            <input v-model="draft.counter_format" placeholder="👥 {count} membres" />
          </label>
        </div>
      </fieldset>

      <!-- ── Anniversaire ── -->
      <fieldset class="card">
        <legend>
          <label class="toggle">
            <input v-model="draft.anniversary_enabled" type="checkbox" />
            🎂 Anniversaire d'arrivée
          </label>
        </legend>
        <div class="grid">
          <label>
            Salon
            <input v-model="draft.anniversary_channel_id" placeholder="ID du salon" />
          </label>
          <label>
            Titre
            <input v-model="draft.anniversary_title" />
          </label>
          <label>
            Image (URL)
            <input v-model="draft.anniversary_image_url" />
          </label>
          <label class="full">
            Message
            <textarea v-model="draft.anniversary_message" rows="3"></textarea>
          </label>
          <label class="full">
            Footer
            <input v-model="draft.anniversary_footer_text" />
          </label>
        </div>
      </fieldset>

      <!-- ── Départ ── -->
      <fieldset class="card">
        <legend>
          <label class="toggle">
            <input v-model="draft.leave_enabled" type="checkbox" />
            👋 Message de départ
          </label>
        </legend>
        <div class="grid">
          <label>
            Salon
            <input v-model="draft.leave_channel_id" placeholder="ID du salon" />
          </label>
          <label>
            Titre
            <input v-model="draft.leave_title" />
          </label>
          <label>
            Image (URL)
            <input v-model="draft.leave_image_url" />
          </label>
          <label class="full">
            Message
            <textarea v-model="draft.leave_message" rows="3"></textarea>
          </label>
          <label class="full">
            Footer
            <input v-model="draft.leave_footer_text" />
          </label>
        </div>
      </fieldset>

      <!-- ── Rejoin (membre déjà venu) ── -->
      <fieldset class="card">
        <legend>🔁 Rejoin (membre déjà venu)</legend>
        <p class="hint">
          Affiché à la place du message de bienvenue si le membre était déjà
          passé sur le serveur.
        </p>
        <div class="grid">
          <label>
            Titre
            <input v-model="draft.rejoin_title" />
          </label>
          <label>
            Image (URL)
            <input v-model="draft.rejoin_image_url" />
          </label>
          <label class="full">
            Message
            <textarea v-model="draft.rejoin_message" rows="3"></textarea>
          </label>
          <label class="full">
            Footer
            <input v-model="draft.rejoin_footer_text" />
          </label>
        </div>
      </fieldset>

      <div class="actions">
        <button type="submit" class="btn-primary" :disabled="saving">
          {{ saving ? "Enregistrement…" : "Enregistrer" }}
        </button>
      </div>
    </form>
  </div>
</template>

<style scoped>
.welcome-page {
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
.lede code {
  background: var(--bg-muted, #2a2a2a);
  padding: 1px 6px;
  border-radius: 3px;
  font-size: 0.9em;
}
.loading,
.empty {
  padding: 48px;
  text-align: center;
  color: var(--text-muted, #888);
}
.welcome-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}
.card {
  background: var(--bg-card, #1f1f1f);
  border: 1px solid var(--border-color, #333);
  border-radius: 8px;
  padding: 20px;
}
.card legend {
  padding: 0 8px;
  font-weight: 600;
}
.toggle {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
  margin-top: 12px;
}
.grid label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.9rem;
}
.grid label.full {
  grid-column: span 2;
}
.grid input,
.grid textarea {
  background: var(--bg-input, #2a2a2a);
  border: 1px solid var(--border-color, #444);
  border-radius: 4px;
  padding: 6px 8px;
  color: inherit;
  font-family: inherit;
}
.grid input[type="color"] {
  height: 36px;
  padding: 2px;
}
.hint {
  font-size: 0.85rem;
  color: var(--text-muted, #888);
  margin: 0 0 8px 0;
}
.dm-details,
.preview {
  margin-top: 12px;
  padding: 8px 12px;
  background: var(--bg-muted, #2a2a2a);
  border-radius: 6px;
}
.preview-embed {
  margin-top: 8px;
  padding: 12px;
  background: var(--bg-card-deep, #181818);
  border-left: 4px solid #5865F2;
  border-radius: 4px;
}
.preview-embed strong {
  display: block;
  margin-bottom: 4px;
}
.preview-embed p {
  margin: 0;
  white-space: pre-wrap;
}
.preview-embed small {
  display: block;
  margin-top: 8px;
  color: var(--text-muted, #888);
  font-size: 0.8rem;
}
.actions {
  display: flex;
  justify-content: flex-end;
  padding: 16px 0;
}
.btn-primary {
  background: #5865F2;
  color: white;
  padding: 10px 24px;
  border: none;
  border-radius: 6px;
  font-weight: 600;
  cursor: pointer;
}
.btn-primary:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}
</style>
