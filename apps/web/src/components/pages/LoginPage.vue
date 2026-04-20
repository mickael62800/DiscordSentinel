<script setup lang="ts">
import { useRouter } from "vue-router";
import { useAuth } from "../../composables/useAuth";
import AppButton from "../atoms/AppButton.vue";

const router = useRouter();
const { loading, error, login, user } = useAuth();

async function handleLogin() {
  await login();
  if (user.value) {
    router.push("/");
  }
}
</script>

<template>
  <div class="login-page">
    <div class="card card--elevated login-card">
      <div class="login-logo">
        <span class="logo-icon">S</span>
      </div>
      <h1>DiscordSentinel</h1>
      <p class="subtitle">Panneau d'administration</p>

      <AppButton
        variant="primary"
        class="discord-btn"
        :disabled="loading"
        @click="handleLogin"
      >
        <svg class="discord-icon" viewBox="0 0 24 24" fill="currentColor" width="20" height="20">
          <path d="M20.317 4.37a19.791 19.791 0 00-4.885-1.515.074.074 0 00-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 00-5.487 0 12.64 12.64 0 00-.617-1.25.077.077 0 00-.079-.037A19.736 19.736 0 003.677 4.37a.07.07 0 00-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 00.031.057 19.9 19.9 0 005.993 3.03.078.078 0 00.084-.028c.462-.63.874-1.295 1.226-1.994a.076.076 0 00-.041-.106 13.107 13.107 0 01-1.872-.892.077.077 0 01-.008-.128 10.2 10.2 0 00.372-.292.074.074 0 01.077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 01.078.01c.12.098.246.198.373.292a.077.077 0 01-.006.127 12.299 12.299 0 01-1.873.892.077.077 0 00-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 00.084.028 19.839 19.839 0 006.002-3.03.077.077 0 00.032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 00-.031-.03z" />
        </svg>
        {{ loading ? "Connexion..." : "Se connecter avec Discord" }}
      </AppButton>

      <p v-if="error" class="error-msg">{{ error }}</p>

      <router-link to="/setup" class="setup-link">
        Configurer la connexion (API, token...)
      </router-link>
    </div>
  </div>
</template>

<style scoped>
.login-page {
  width: 100vw;
  height: 100vh;
  display: flex;
  align-items: center;
  justify-content: center;
  background: linear-gradient(135deg, var(--bg-primary), var(--bg-secondary));
}

.login-card {
  padding: var(--space-3xl);
  border-radius: var(--radius-xl);
  box-shadow: var(--shadow-lg);
  text-align: center;
  width: 380px;
}

.login-logo {
  margin-bottom: var(--space-lg);
}

.logo-icon {
  display: inline-flex;
  width: 64px;
  height: 64px;
  background: linear-gradient(135deg, var(--accent), var(--accent-alt));
  border-radius: var(--radius-xl);
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 32px;
  color: white;
}

h1 {
  font-size: 24px;
  font-weight: 700;
  margin-bottom: var(--space-xs);
}

.subtitle {
  color: var(--text-secondary);
  font-size: 14px;
  margin-bottom: var(--space-2xl);
}

.discord-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-sm);
  padding: var(--space-md) var(--space-xl);
  font-size: 15px;
  font-weight: 600;
  background-color: var(--accent);
  border-radius: var(--radius-md);
  transition: background-color var(--transition-base);
}

.discord-btn:hover:not(:disabled) {
  background-color: var(--accent-hover);
}

.discord-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
}

.discord-icon {
  width: 20px;
  height: 20px;
}

.error-msg {
  margin-top: var(--space-lg);
  color: var(--danger);
  font-size: 13px;
}

.setup-link {
  display: inline-block;
  margin-top: var(--space-lg);
  color: var(--text-secondary);
  font-size: 13px;
  text-decoration: none;
  transition: color var(--transition-base);
}

.setup-link:hover {
  color: var(--accent);
  text-decoration: underline;
}
</style>
