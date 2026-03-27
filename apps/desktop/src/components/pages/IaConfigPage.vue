<script setup lang="ts">
import { ref, watch } from "vue";
import { useIaConfig } from "../../composables/useIaConfig";
import { useGuildSelector } from "../../composables/useGuildSelector";

const { config, loading, saving, error, saveConfig } = useIaConfig();
const { guildIdFilter } = useGuildSelector();

const textEnabled = ref(true);
const textThreshold = ref(0.5);
const visionEnabled = ref(true);
const visionThreshold = ref(0.5);

watch(config, (c) => {
  if (c) {
    textEnabled.value = c.text_enabled;
    textThreshold.value = c.text_threshold;
    visionEnabled.value = c.vision_enabled;
    visionThreshold.value = c.vision_threshold;
  }
});

async function handleSave() {
  await saveConfig({
    text_enabled: textEnabled.value,
    text_threshold: textThreshold.value,
    vision_enabled: visionEnabled.value,
    vision_threshold: visionThreshold.value,
  });
}
</script>

<template>
  <div class="ia-config-page">
    <h1>Configuration IA</h1>
    <p class="subtitle">Seuils de confiance pour l'inference IA par serveur</p>

    <div v-if="!guildIdFilter" class="no-guild">
      Selectionnez un serveur pour configurer les seuils IA.
    </div>

    <div v-else-if="loading" class="loading">Chargement...</div>

    <div v-else class="config-sections">
      <!-- Text Inference -->
      <section class="config-section">
        <div class="section-header">
          <h2>Analyse de texte (sentiments)</h2>
          <label class="toggle">
            <input type="checkbox" v-model="textEnabled" />
            <span>{{ textEnabled ? "Active" : "Desactive" }}</span>
          </label>
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
          <label class="toggle">
            <input type="checkbox" v-model="visionEnabled" />
            <span>{{ visionEnabled ? "Active" : "Desactive" }}</span>
          </label>
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
</style>
