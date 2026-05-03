<script setup lang="ts">
import { useWelcome } from "@/composables/useWelcome";
import WelcomeForm from "@/components/organisms/WelcomeForm.vue";

const { config, loading } = useWelcome();
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
    <WelcomeForm v-else />
  </div>
</template>

<style scoped>
.welcome-page {
  max-width: 1100px;
  margin: 0 auto;
}
.page-header { margin-bottom: 24px; }
.page-header h1 {
  margin: 0 0 8px 0;
  font-size: 22px;
}
.lede {
  color: var(--text-secondary);
  margin: 0;
  font-size: 13px;
  line-height: 1.5;
}
.lede code {
  background: var(--bg-card);
  border: 1px solid var(--border);
  padding: 1px 6px;
  border-radius: var(--radius-sm, 6px);
  font-size: 0.9em;
  font-family: "JetBrains Mono", monospace;
  color: var(--accent);
}
.loading,
.empty {
  padding: 48px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
}
</style>
