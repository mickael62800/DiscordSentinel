<script setup lang="ts">
import { ref } from "vue";
import { useRouter } from "vue-router";
import { configService } from "@/services/configService";
import { useAuth } from "../../composables/useAuth";
import { resetApiBaseUrlCache } from "../../utils/api";
import AppButton from "../atoms/AppButton.vue";

const router = useRouter();
const { saveConfig } = useAuth();

const step = ref(1);

// Step 1: API Backend
const apiUrl = ref("http://localhost:3000");
const apiKey = ref("");

// Step 2: Discord OAuth
const clientId = ref("");
const clientSecret = ref("");

const saving = ref(false);
const error = ref<string | null>(null);

async function nextStep() {
  if (!apiUrl.value.trim()) {
    error.value = "L'URL de l'API est requise.";
    return;
  }

  saving.value = true;
  error.value = null;
  try {
    configService.saveApiConfig(apiUrl.value.trim(), apiKey.value.trim());
    resetApiBaseUrlCache();
    step.value = 2;
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}

async function handleSubmit() {
  if (!clientId.value.trim() || !clientSecret.value.trim()) {
    error.value = "Les deux champs sont requis.";
    return;
  }

  saving.value = true;
  error.value = null;
  try {
    await saveConfig(clientId.value.trim(), clientSecret.value.trim());
    router.push("/login");
  } catch (e) {
    error.value = String(e);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div class="setup-page">
    <div class="setup-card">
      <div class="setup-logo">
        <span class="logo-icon">S</span>
      </div>
      <h1>DiscordSentinel</h1>
      <p class="subtitle">Configuration initiale</p>

      <div class="steps">
        <span :class="['step', { active: step === 1, done: step > 1 }]">1. API Backend</span>
        <span class="step-sep">&rarr;</span>
        <span :class="['step', { active: step === 2 }]">2. OAuth Discord</span>
      </div>

      <!-- Step 1: API Config -->
      <form v-if="step === 1" class="setup-form" @submit.prevent="nextStep">
        <div class="setup-instructions">
          <p>Entrez les details de connexion de votre backend <strong>Sentinel API</strong>.</p>
        </div>

        <div class="field">
          <label for="api-url">API URL</label>
          <input
            id="api-url"
            v-model="apiUrl"
            type="text"
            placeholder="http://localhost:3000"
            autocomplete="off"
          />
        </div>

        <div class="field">
          <label for="api-key">Cle API (jeton Bearer)</label>
          <input
            id="api-key"
            v-model="apiKey"
            type="password"
            placeholder="Laisser vide si l'authentification est desactivee"
            autocomplete="off"
          />
          <span class="field-hint">La cle API_KEY configuree dans votre fichier .env backend</span>
        </div>

        <p v-if="error" class="error-msg">{{ error }}</p>

        <AppButton
          variant="primary"
          class="save-btn"
          :disabled="saving || !apiUrl.trim()"
        >
          {{ saving ? "Enregistrement..." : "Suivant" }}
        </AppButton>
      </form>

      <!-- Step 2: Discord Config -->
      <form v-else class="setup-form" @submit.prevent="handleSubmit">
        <div class="setup-instructions">
          <p>Creez une application sur le <strong>portail developpeur Discord</strong> et entrez les identifiants ci-dessous.</p>
          <p class="hint">URI de redirection : <code>http://localhost:19836/callback</code></p>
        </div>

        <div class="field">
          <label for="client-id">Client ID</label>
          <input
            id="client-id"
            v-model="clientId"
            type="text"
            placeholder="e.g. 123456789012345678"
            autocomplete="off"
          />
        </div>

        <div class="field">
          <label for="client-secret">Client Secret</label>
          <input
            id="client-secret"
            v-model="clientSecret"
            type="password"
            placeholder="Votre client secret"
            autocomplete="off"
          />
        </div>

        <p v-if="error" class="error-msg">{{ error }}</p>

        <div class="btn-row">
          <button type="button" class="back-link" @click="step = 1">&larr; Retour</button>
          <AppButton
            variant="primary"
            class="save-btn"
            :disabled="saving || !clientId.trim() || !clientSecret.trim()"
          >
            {{ saving ? "Enregistrement..." : "Enregistrer et connecter" }}
          </AppButton>
        </div>
      </form>
    </div>
  </div>
</template>

<style scoped>
.setup-page {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--bg-primary), var(--bg-secondary));
}

.setup-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 48px;
  text-align: center;
  width: 460px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}

.setup-logo {
  margin-bottom: 20px;
}

.logo-icon {
  display: inline-flex;
  width: 64px;
  height: 64px;
  background: linear-gradient(135deg, var(--accent), #7c5cfc);
  border-radius: 16px;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 32px;
  color: white;
}

h1 {
  font-size: 24px;
  font-weight: 700;
  margin-bottom: 4px;
}

.subtitle {
  color: var(--text-secondary);
  font-size: 14px;
  margin-bottom: 20px;
}

.steps {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 8px;
  margin-bottom: 24px;
  font-size: 13px;
}

.step {
  color: var(--text-secondary);
  opacity: 0.5;
}

.step.active {
  color: var(--accent);
  opacity: 1;
  font-weight: 600;
}

.step.done {
  color: var(--success);
  opacity: 0.8;
}

.step-sep {
  color: var(--text-secondary);
  opacity: 0.3;
}

.setup-instructions {
  text-align: left;
  margin-bottom: 20px;
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.6;
}

.setup-instructions strong {
  color: var(--text-primary);
}

.hint {
  margin-top: 8px;
}

.hint code {
  background-color: var(--bg-hover);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
  color: var(--accent);
  user-select: all;
}

.setup-form {
  display: flex;
  flex-direction: column;
  gap: 16px;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
  text-align: left;
}

.field label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
}

.field input {
  width: 100%;
}

.field-hint {
  font-size: 11px;
  color: var(--text-secondary);
  opacity: 0.7;
}

.error-msg {
  color: var(--danger);
  font-size: 13px;
  text-align: left;
}

.save-btn {
  width: 100%;
  padding: 12px 24px;
  font-size: 15px;
  font-weight: 600;
}

.save-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.btn-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
}

.btn-row .save-btn {
  flex: 1;
}

.back-link {
  background: none;
  color: var(--text-secondary);
  font-size: 13px;
  padding: 8px 0;
}

.back-link:hover {
  color: var(--text-primary);
}
</style>
