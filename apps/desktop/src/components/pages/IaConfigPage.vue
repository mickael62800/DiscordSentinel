<script setup lang="ts">
import { ref, watch } from "vue";
import { useIaConfig } from "../../composables/useIaConfig";
import { useGuildSelector } from "../../composables/useGuildSelector";
import AppToggle from "../atoms/AppToggle.vue";

const { config, loading, saving, error, saveConfig } = useIaConfig();
const { guildIdFilter } = useGuildSelector();

const textEnabled = ref(true);
const textThreshold = ref(0.5);
const visionEnabled = ref(true);
const visionThreshold = ref(0.5);
const contextDampening = ref(0.65);
const contextFormat = ref("natural");
const contextMaxMessages = ref(3);
const contextMaxChars = ref(200);

watch(config, (c) => {
  if (c) {
    textEnabled.value = c.text_enabled;
    textThreshold.value = c.text_threshold;
    visionEnabled.value = c.vision_enabled;
    visionThreshold.value = c.vision_threshold;
    contextDampening.value = c.context_dampening;
    contextFormat.value = c.context_format;
    contextMaxMessages.value = c.context_max_messages;
    contextMaxChars.value = c.context_max_chars;
  }
});

async function handleSave() {
  await saveConfig({
    text_enabled: textEnabled.value,
    text_threshold: textThreshold.value,
    vision_enabled: visionEnabled.value,
    vision_threshold: visionThreshold.value,
    context_dampening: contextDampening.value,
    context_format: contextFormat.value,
    context_max_messages: contextMaxMessages.value,
    context_max_chars: contextMaxChars.value,
  });
}
</script>

<template>
  <div class="ia-config-page">
    <div class="page-header-row">
      <div>
        <h1>Configuration IA</h1>
        <p class="subtitle">Seuils de confiance pour l'inference IA par serveur</p>
      </div>
      <router-link to="/ai-training" class="cross-link">&larr; Entrainement des modeles</router-link>
    </div>

    <div v-if="!guildIdFilter" class="no-guild">
      Selectionnez un serveur pour configurer les seuils IA.
    </div>

    <div v-else-if="loading" class="loading">Chargement...</div>

    <div v-else class="config-sections">
      <!-- Text Inference -->
      <section class="config-section">
        <div class="section-header">
          <h2>Analyse de texte (sentiments)</h2>
          <AppToggle :modelValue="textEnabled" @update:modelValue="textEnabled = $event" />
        </div>
        <p class="description">
          Detection de colere, rage, menaces et harcelement via DistilBERT.
          Un seuil bas detecte plus de contenus mais augmente les faux positifs.
        </p>
        <div class="slider-group" :class="{ disabled: !textEnabled }">
          <label>
            Seuil de confiance : <strong>{{ (textThreshold * 100).toFixed(0) }}%</strong>
          </label>
          <input
            type="range"
            min="0.1"
            max="0.95"
            step="0.05"
            v-model.number="textThreshold"
            :disabled="!textEnabled"
          />
          <div class="range-labels">
            <span>10% (sensible)</span>
            <span>50% (defaut)</span>
            <span>95% (strict)</span>
          </div>
        </div>
      </section>

      <!-- Vision Inference -->
      <section class="config-section">
        <div class="section-header">
          <h2>Analyse d'images (vision)</h2>
          <AppToggle :modelValue="visionEnabled" @update:modelValue="visionEnabled = $event" />
        </div>
        <p class="description">
          Detection NSFW et contenus illicites via EfficientNetV2.
          Un seuil bas detecte plus d'images mais augmente les faux positifs.
        </p>
        <div class="slider-group" :class="{ disabled: !visionEnabled }">
          <label>
            Seuil de confiance : <strong>{{ (visionThreshold * 100).toFixed(0) }}%</strong>
          </label>
          <input
            type="range"
            min="0.1"
            max="0.95"
            step="0.05"
            v-model.number="visionThreshold"
            :disabled="!visionEnabled"
          />
          <div class="range-labels">
            <span>10% (sensible)</span>
            <span>50% (defaut)</span>
            <span>95% (strict)</span>
          </div>
        </div>
      </section>

      <!-- Contexte conversationnel -->
      <section class="config-section">
        <div class="section-header">
          <h2>Contexte conversationnel</h2>
        </div>
        <p class="description">
          Envoie les messages precedents du canal au modele IA pour reduire les faux positifs
          (blagues entre amis, contexte de conversation).
        </p>

        <div class="field-group">
          <label>
            Attenuation du score IA : <strong>{{ (contextDampening * 100).toFixed(0) }}%</strong>
          </label>
          <input
            type="range"
            min="0.1"
            max="1.0"
            step="0.05"
            v-model.number="contextDampening"
          />
          <div class="range-labels">
            <span>10% (fort dampening)</span>
            <span>65% (defaut)</span>
            <span>100% (aucun dampening)</span>
          </div>
        </div>

        <div class="field-group">
          <label>Format du contexte</label>
          <select v-model="contextFormat" class="select-field">
            <option value="natural">Naturel (conversation brute)</option>
            <option value="tagged">Tagged (balises [message]/[context])</option>
          </select>
        </div>

        <div class="field-row">
          <div class="field-group">
            <label>Messages de contexte</label>
            <input type="number" min="0" max="10" v-model.number="contextMaxMessages" class="number-field" />
          </div>
          <div class="field-group">
            <label>Caracteres max / message</label>
            <input type="number" min="50" max="500" step="50" v-model.number="contextMaxChars" class="number-field" />
          </div>
        </div>
      </section>

      <div v-if="error" class="error">{{ error }}</div>

      <button class="save-btn" @click="handleSave" :disabled="saving">
        {{ saving ? "Sauvegarde..." : "Sauvegarder" }}
      </button>
    </div>
  </div>
</template>

<style scoped>
.ia-config-page {
  padding: 2rem;
  max-width: 700px;
}

.subtitle {
  color: var(--text-secondary, #888);
  margin-bottom: 2rem;
}

.no-guild, .loading {
  padding: 2rem;
  text-align: center;
  color: var(--text-secondary, #888);
}

.config-sections {
  display: flex;
  flex-direction: column;
  gap: 2rem;
}

.config-section {
  background: var(--bg-secondary, #1e1e2e);
  border-radius: 12px;
  padding: 1.5rem;
}

.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
}

.section-header h2 {
  margin: 0;
  font-size: 1.1rem;
}

.toggle {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  cursor: pointer;
  font-size: 0.85rem;
}

.description {
  color: var(--text-secondary, #888);
  font-size: 0.85rem;
  margin-bottom: 1rem;
}

.slider-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.slider-group.disabled {
  opacity: 0.4;
  pointer-events: none;
}

.slider-group label {
  font-size: 0.9rem;
}

.slider-group input[type="range"] {
  width: 100%;
  accent-color: var(--accent, #7c3aed);
}

.range-labels {
  display: flex;
  justify-content: space-between;
  font-size: 0.75rem;
  color: var(--text-secondary, #888);
}

.error {
  color: #ef4444;
  font-size: 0.85rem;
  padding: 0.5rem;
  background: rgba(239, 68, 68, 0.1);
  border-radius: 6px;
}

.save-btn {
  padding: 0.75rem 2rem;
  background: var(--accent, #7c3aed);
  color: white;
  border: none;
  border-radius: 8px;
  cursor: pointer;
  font-size: 0.9rem;
  align-self: flex-start;
}

.save-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.save-btn:hover:not(:disabled) {
  opacity: 0.9;
}

.field-group {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  margin-bottom: 1rem;
}

.field-group label {
  font-size: 0.9rem;
}

.field-row {
  display: flex;
  gap: 1.5rem;
}

.field-row .field-group {
  flex: 1;
}

.select-field, .number-field {
  padding: 0.5rem 0.75rem;
  background: var(--bg-primary, #111);
  color: var(--text-primary, #eee);
  border: 1px solid var(--border, #333);
  border-radius: 6px;
  font-size: 0.85rem;
}

.select-field:focus, .number-field:focus {
  outline: none;
  border-color: var(--accent, #7c3aed);
}

/* Cross-link */
.page-header-row { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 1rem; }
.cross-link { font-size: 13px; font-weight: 600; color: var(--accent); text-decoration: none; padding: 8px 16px; border: 1px solid var(--accent); border-radius: 8px; white-space: nowrap; transition: all 0.15s; }
.cross-link:hover { background: var(--accent); color: white; }
</style>
