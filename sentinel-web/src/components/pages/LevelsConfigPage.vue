<script setup lang="ts">
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import LevelsConfigCard from "../organisms/LevelsConfigCard.vue";
import LevelsManualXpCard from "../organisms/LevelsManualXpCard.vue";
</script>

<template>
  <AdminPageShell title="Niveaux — Configuration" icon="⚙️">
    <template #lede>
      Paramètres XP / niveau (texte + vocal).
    </template>

    <div class="grid">
      <LevelsConfigCard />
      <LevelsManualXpCard />
    </div>
  </AdminPageShell>
</template>

<style scoped>
@import "./_admin-page-shared.css";
.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
@media (max-width: 640px) {
  .grid { grid-template-columns: 1fr; gap: 12px; }
}
</style>
