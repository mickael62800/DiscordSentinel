<script setup lang="ts">
import { useTauntsConfig } from "@/composables/useTauntsConfig";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import TauntsConfigCard from "../organisms/TauntsConfigCard.vue";
import TauntsOptOutsCard from "../organisms/TauntsOptOutsCard.vue";

const { loading, error, fetchConfig } = useTauntsConfig();
</script>

<template>
  <div class="taunts-page page--xs">
    <header class="page-header">
      <h1>🔥 Railleries automatiques</h1>
      <p class="subtitle">
        Systeme transversal a tous les jeux (Coup de Coude, Blackjack,
        economie). Configure le salon ou les railleries sont postees et
        la liste des joueurs qui ont opt-out via <code>/no-taunts on</code>.
      </p>
    </header>

    <LoadingState v-if="loading" message="Chargement…" />
    <ErrorState v-else-if="error" :message="error" @retry="fetchConfig" />

    <template v-else>
      <TauntsConfigCard />
      <TauntsOptOutsCard />
    </template>
  </div>
</template>

<style scoped>
.taunts-page {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 24px;
}
.page-header h1 { margin: 0 0 8px; font-size: 28px; }
.subtitle {
  margin: 0;
  color: var(--text-secondary);
  font-size: 14px;
  line-height: 1.5;
}
.subtitle code {
  background: var(--bg-elevated);
  padding: 2px 6px;
  border-radius: 4px;
  font-size: 12px;
}
</style>
