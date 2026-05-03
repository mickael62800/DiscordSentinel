<script setup lang="ts">
import LevelsConfigCard from "../organisms/LevelsConfigCard.vue";
import LevelsManualXpCard from "../organisms/LevelsManualXpCard.vue";
import ConductConfigCard from "../organisms/ConductConfigCard.vue";
import ConductActionsCard from "../organisms/ConductActionsCard.vue";
</script>

<template>
  <div class="page page--constrained">
    <header class="page-header">
      <h1>⚙️ Levels & Conduite — Configuration</h1>
      <p class="lede">
        Paramètres XP / niveau (texte + vocal) et système de conduite
        (points, regen, escalade automatique vers ban à 0 point).
      </p>
    </header>

    <div class="grid">
      <LevelsConfigCard />
      <LevelsManualXpCard />
      <ConductConfigCard />
      <ConductActionsCard />
    </div>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
@media (max-width: 640px) {
  .grid { grid-template-columns: 1fr; gap: 12px; }
}
</style>
