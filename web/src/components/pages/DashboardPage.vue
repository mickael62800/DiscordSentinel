<script setup lang="ts">
import SectionCard from "../molecules/SectionCard.vue";
import DashboardHero from "../organisms/DashboardHero.vue";
import { useDashboardSections } from "@/composables/useDashboardSections";
import { useUniverse } from "@/composables/useUniverse";

// Les tuiles suivent le meme univers que la barre laterale : sans ce filtre,
// les entrees Nexus apparaissaient en bas du tableau de bord Sentinel.
const { universe } = useUniverse();
const { groups } = useDashboardSections(universe);
</script>

<template>
  <div class="home page--constrained">
    <DashboardHero />

    <section v-for="g in groups" :key="g.prefix" class="section-group">
      <h2 class="section-group__title">{{ g.label }}</h2>
      <div class="section-grid">
        <SectionCard
          v-for="s in g.sections"
          :key="s.key"
          :path="s.path"
          :label="s.label"
          :icon="s.icon"
          :section-key="s.key"
          :required-bot="s.requiredBot"
          :required-any-bot="s.requiredAnyBot"
        />
      </div>
    </section>
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

.section-group {
  margin-top: 20px;
}

.section-group:first-of-type {
  margin-top: 8px;
}

.section-group__title {
  margin: 0 0 10px;
  font-size: 0.8rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  color: var(--text-muted, #8b93a7);
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
