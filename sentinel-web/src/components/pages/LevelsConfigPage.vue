<script setup lang="ts">
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import LevelsManualXpCard from "../organisms/LevelsManualXpCard.vue";
</script>

<template>
  <AdminPageShell title="Niveaux — XP manuelle" icon="⚙️">
    <template #lede>
      Ajustement manuel de l'XP des membres (admin). La configuration XP
      (gains, cooldown, message de level-up…) se règle sur la page du
      composant « progression-bot ».
    </template>

    <div class="grid">
      <LevelsManualXpCard />
    </div>
  </AdminPageShell>
</template>

<style scoped>
@import "./_admin-page-shared.css";
.grid { display: grid; grid-template-columns: 1fr; gap: 16px; }
</style>
