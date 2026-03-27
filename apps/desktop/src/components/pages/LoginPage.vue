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
    <div class="login-card">
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
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 16px;
  padding: 48px;
  text-align: center;
  width: 380px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.3);
}

.login-logo {
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
  margin-bottom: 32px;
}

.discord-btn {
  width: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 10px;
  padding: 12px 24px;
  font-size: 15px;
  font-weight: 600;
  background-color: #5865f2;
  border-radius: 8px;
  transition: background-color 0.2s;
}

.discord-btn:hover:not(:disabled) {
  background-color: #4752c4;
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
  margin-top: 16px;
  color: var(--danger);
  font-size: 13px;
}
</style>
