<script setup lang="ts">
import { computed, reactive, watch } from "vue";
import { useWelcome } from "@/composables/useWelcome";
import { useGuildSelector } from "@/composables/useGuildSelector";
import AppToggle from "@/components/atoms/AppToggle.vue";
import ChannelSelect from "@/components/atoms/ChannelSelect.vue";
import RoleSelect from "@/components/atoms/RoleSelect.vue";

const { config, saving, saveConfig } = useWelcome();
const { guildIdFilter } = useGuildSelector();

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
  <form class="welcome-form" @submit.prevent="onSave">
    <!-- Welcome -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.welcome_enabled" />
          <span>Message de bienvenue actif</span>
        </label>
      </legend>
      <div class="grid" :class="{ 'grid--disabled': !draft.welcome_enabled }">
        <label>Salon
          <ChannelSelect v-model="draft.welcome_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Titre embed
          <input v-model="draft.welcome_title" placeholder="Bienvenue !" />
        </label>
        <label>Couleur (hex)
          <input v-model="draft.welcome_embed_color" type="color" />
        </label>
        <label>Image (URL)
          <input v-model="draft.welcome_image_url" placeholder="https://..." />
        </label>
        <label class="full">Message
          <textarea v-model="draft.welcome_message" rows="6"></textarea>
        </label>
        <label class="full">Footer
          <input v-model="draft.welcome_footer_text" />
        </label>
      </div>

      <details class="dm-details">
        <summary>DM de bienvenue (optionnel)</summary>
        <label class="toggle-row">
          <AppToggle v-model="draft.welcome_dm_enabled" />
          <span>Activer le DM</span>
        </label>
        <label class="full">Message DM
          <textarea v-model="draft.welcome_dm_message" rows="6"></textarea>
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

    <!-- Verification gate (rules) -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.rules_enabled" />
          <span>🔒 Verification gate (lecture des règles)</span>
        </label>
      </legend>
      <p class="hint">
        Affiche un bouton « J'ai lu les règles » dans le salon dédié.
        Le rôle configuré est attribué après acceptation.
      </p>
      <div class="grid" :class="{ 'grid--disabled': !draft.rules_enabled }">
        <label>Salon des règles
          <ChannelSelect v-model="draft.rules_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Rôle attribué
          <RoleSelect v-model="draft.rules_role_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Texte du bouton
          <input v-model="draft.rules_button_label" placeholder="J'ai lu les règles" />
        </label>
        <label class="full">Message
          <textarea v-model="draft.rules_message" rows="6"></textarea>
        </label>
      </div>
    </fieldset>

    <!-- Compteur de membres -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.counter_enabled" />
          <span>🔢 Compteur de membres</span>
        </label>
      </legend>
      <div class="grid" :class="{ 'grid--disabled': !draft.counter_enabled }">
        <label>Salon
          <ChannelSelect v-model="draft.counter_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Format
          <input v-model="draft.counter_format" placeholder="👥 {count} membres" />
        </label>
      </div>
    </fieldset>

    <!-- Anniversaire -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.anniversary_enabled" />
          <span>🎂 Anniversaire d'arrivée</span>
        </label>
      </legend>
      <div class="grid" :class="{ 'grid--disabled': !draft.anniversary_enabled }">
        <label>Salon
          <ChannelSelect v-model="draft.anniversary_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Titre
          <input v-model="draft.anniversary_title" />
        </label>
        <label>Image (URL)
          <input v-model="draft.anniversary_image_url" />
        </label>
        <label class="full">Message
          <textarea v-model="draft.anniversary_message" rows="6"></textarea>
        </label>
        <label class="full">Footer
          <input v-model="draft.anniversary_footer_text" />
        </label>
      </div>
    </fieldset>

    <!-- Départ -->
    <fieldset class="card">
      <legend>
        <label class="toggle-row">
          <AppToggle v-model="draft.leave_enabled" />
          <span>👋 Message de départ</span>
        </label>
      </legend>
      <div class="grid" :class="{ 'grid--disabled': !draft.leave_enabled }">
        <label>Salon
          <ChannelSelect v-model="draft.leave_channel_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>Titre
          <input v-model="draft.leave_title" />
        </label>
        <label>Image (URL)
          <input v-model="draft.leave_image_url" />
        </label>
        <label class="full">Message
          <textarea v-model="draft.leave_message" rows="6"></textarea>
        </label>
        <label class="full">Footer
          <input v-model="draft.leave_footer_text" />
        </label>
      </div>
    </fieldset>

    <!-- Rejoin -->
    <fieldset class="card">
      <legend>🔁 Rejoin (membre déjà venu)</legend>
      <p class="hint">
        Affiché à la place du message de bienvenue si le membre était déjà
        passé sur le serveur.
      </p>
      <div class="grid">
        <label>Titre
          <input v-model="draft.rejoin_title" />
        </label>
        <label>Image (URL)
          <input v-model="draft.rejoin_image_url" />
        </label>
        <label class="full">Message
          <textarea v-model="draft.rejoin_message" rows="6"></textarea>
        </label>
        <label class="full">Footer
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
</template>

<style scoped>
.welcome-form {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* Cards (fieldsets) */
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg, 12px);
  padding: 20px;
}
.card legend {
  padding: 0 8px;
  font-weight: 700;
  font-size: 14px;
}

.toggle-row {
  display: inline-flex;
  align-items: center;
  gap: 10px;
  cursor: pointer;
  font-weight: 600;
  font-size: 14px;
}

/* Inputs / textarea */
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 14px;
  margin-top: 14px;
}
.grid label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.6px;
  color: var(--text-secondary);
}
.grid label.full { grid-column: span 2; }

.grid input,
.grid textarea {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md, 8px);
  padding: 8px 12px;
  color: var(--text-primary);
  font-family: inherit;
  font-size: 13px;
  font-weight: 500;
  text-transform: none;
  letter-spacing: 0;
  outline: none;
  transition: border-color var(--transition-fast, 0.15s),
    box-shadow var(--transition-fast, 0.15s);
}
.grid input:hover,
.grid textarea:hover {
  border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
}
.grid input:focus,
.grid textarea:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent);
}
.grid textarea {
  resize: vertical;
  min-height: 140px;
}
.grid--disabled {
  opacity: 0.45;
  pointer-events: none;
  filter: grayscale(0.3);
}
.grid input[type="color"] {
  height: 38px;
  padding: 3px;
  cursor: pointer;
}

.hint {
  font-size: 12px;
  color: var(--text-secondary);
  margin: 0 0 8px 0;
  line-height: 1.5;
}

/* Details (DM, preview) */
.dm-details,
.preview {
  margin-top: 14px;
  padding: 10px 14px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md, 8px);
}
.dm-details summary,
.preview summary {
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  user-select: none;
}
.dm-details summary:hover,
.preview summary:hover {
  color: var(--text-primary);
}
.dm-details[open] summary,
.preview[open] summary {
  margin-bottom: 10px;
}

.preview-embed {
  margin-top: 8px;
  padding: 14px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-left: 4px solid var(--accent);
  border-radius: var(--radius-md, 8px);
}
.preview-embed strong {
  display: block;
  margin-bottom: 6px;
  font-size: 14px;
  color: var(--text-primary);
}
.preview-embed p {
  margin: 0;
  white-space: pre-wrap;
  font-size: 13px;
  color: var(--text-primary);
  line-height: 1.5;
}
.preview-embed small {
  display: block;
  margin-top: 8px;
  color: var(--text-secondary);
  font-size: 11px;
}

/* Actions */
.actions {
  display: flex;
  justify-content: flex-end;
  padding: 8px 0 0;
}
.btn-primary {
  background: var(--accent);
  color: white;
  padding: 10px 24px;
  border: 1px solid transparent;
  border-radius: var(--radius-md, 8px);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: background-color var(--transition-fast, 0.15s),
    box-shadow var(--transition-fast, 0.15s);
}
.btn-primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--accent) 88%, white);
  box-shadow: 0 4px 14px color-mix(in srgb, var(--accent) 35%, transparent);
}
.btn-primary:disabled {
  opacity: 0.55;
  cursor: not-allowed;
  box-shadow: none;
}
</style>
