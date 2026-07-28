<script setup lang="ts">
import { useWelcome } from "@/composables/useWelcome";
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import WelcomeForm from "@/components/organisms/WelcomeForm.vue";

const { config, loading } = useWelcome();
</script>

<template>
  <AdminPageShell title="Welcome / Onboarding" icon="👋">
    <template #lede>
      Configure le message de bienvenue, le verification gate (règles à
      accepter), le compteur de membres, l'anniversaire d'arrivée, et le
      message de départ. Variables disponibles dans les messages :
      <code>{user}</code>, <code>{server}</code>, <code>{count}</code>.
    </template>

    <div v-if="loading" class="loading">Chargement…</div>
    <div v-else-if="!config" class="empty">
      Sélectionne une guild dans le menu en haut pour configurer.
    </div>
    <WelcomeForm v-else />
  </AdminPageShell>
</template>

<style scoped>
.loading,
.empty {
  padding: 48px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
}
</style>
