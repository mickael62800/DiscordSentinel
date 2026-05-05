<script setup lang="ts">
import { useTauntsConfig } from "@/composables/useTauntsConfig";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import AdminPageShell from "../layouts/AdminPageShell.vue";
import TauntsConfigCard from "../organisms/TauntsConfigCard.vue";
import TauntsOptOutsCard from "../organisms/TauntsOptOutsCard.vue";

const { loading, error, fetchConfig } = useTauntsConfig();
</script>

<template>
  <AdminPageShell title="Railleries automatiques" icon="🔥">
    <template #lede>
      Systeme transversal a tous les jeux (Coup de Coude, Blackjack,
      economie). Configure le salon ou les railleries sont postees et
      la liste des joueurs qui ont opt-out via <code>/no-taunts on</code>.
    </template>

    <LoadingState v-if="loading" message="Chargement…" />
    <ErrorState v-else-if="error" :message="error" @retry="fetchConfig" />

    <template v-else>
      <TauntsConfigCard />
      <TauntsOptOutsCard />
    </template>
  </AdminPageShell>
</template>
