<script setup lang="ts">
import SectionCard from "../molecules/SectionCard.vue";
import DashboardHero from "../organisms/DashboardHero.vue";
import { useDashboardSections } from "@/composables/useDashboardSections";

const { sections } = useDashboardSections();
</script>

<template>
  <div class="home page--wide">
    <DashboardHero />

    <div class="section-grid">
      <SectionCard
        v-for="s in sections"
        :key="s.key"
        :path="s.path"
        :label="s.label"
        :icon="s.icon"
        :section-key="s.key"
        :required-bot="s.requiredBot"
        :required-any-bot="s.requiredAnyBot"
      />
    </div>
  </div>
</template>

<style scoped>
.home {
  height: 100%;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: auto;
}

.section-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
  grid-auto-rows: 120px;
  gap: 12px;
}

@media (max-width: 640px) {
  /* Grille 2 colonnes fixes sur mobile (sinon avec auto-fill on peut
     tomber a 1 col tres etroite + 1 col vide). */
  .section-grid {
    grid-template-columns: repeat(2, minmax(0, 1fr));
    grid-auto-rows: 100px;
    gap: 8px;
  }
}

@media (max-width: 380px) {
  .section-grid { grid-auto-rows: 90px; gap: 6px; }
}
</style>
