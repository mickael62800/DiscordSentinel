<script setup lang="ts">
import { ref, computed } from "vue";
import type { Game } from "@/services/gamesService";
import { useGames } from "@/composables/useGames";
import { useGuildSelector } from "@/composables/useGuildSelector";
import AppButton from "@/components/atoms/AppButton.vue";
import EmptyState from "@/components/atoms/EmptyState.vue";
import LoadingState from "@/components/atoms/LoadingState.vue";
import GamesTable from "@/components/organisms/GamesTable.vue";
import GamePanelsList from "@/components/organisms/GamePanelsList.vue";
import GameFormModal from "@/components/organisms/GameFormModal.vue";

const { selectedGuildId } = useGuildSelector();
const { games, panels, categories, loading } = useGames();

const selectedCategory = ref<string>("__all__");

const filteredGames = computed<Game[]>(() => {
  if (selectedCategory.value === "__all__") return games.value;
  if (selectedCategory.value === "__none__") {
    return games.value.filter((g) => !g.category || !g.category.trim());
  }
  return games.value.filter(
    (g) => (g.category ?? "").toLowerCase() === selectedCategory.value.toLowerCase(),
  );
});

const modalOpen = ref(false);
const modalTarget = ref<Game | null>(null);

function openCreate() {
  modalTarget.value = null;
  modalOpen.value = true;
}

function openEdit(game: Game) {
  modalTarget.value = game;
  modalOpen.value = true;
}

function closeModal() {
  modalOpen.value = false;
}

const defaultCategory = computed(() =>
  selectedCategory.value !== "__all__" && selectedCategory.value !== "__none__"
    ? selectedCategory.value
    : "",
);
</script>

<template>
  <div class="games-page page--constrained">
    <div class="page-header">
      <h1>Gestion des jeux</h1>
      <div class="header-actions">
        <select
          v-model="selectedCategory"
          class="category-select"
          :disabled="!selectedGuildId"
        >
          <option value="__all__">Toutes les categories</option>
          <option value="__none__">(Sans categorie)</option>
          <option v-for="c in categories" :key="c" :value="c">{{ c }}</option>
        </select>
        <AppButton variant="primary" :disabled="!selectedGuildId" @click="openCreate">
          + Ajouter un jeu
        </AppButton>
      </div>
    </div>

    <EmptyState
      v-if="!selectedGuildId"
      message="Selectionnez un serveur pour gerer les jeux."
    />
    <LoadingState v-else-if="loading" />
    <template v-else>
      <EmptyState
        v-if="filteredGames.length === 0"
        message="Aucun jeu dans cette categorie. Cliquez sur « Ajouter un jeu » pour commencer."
      />
      <GamesTable v-else :games="filteredGames" @edit="openEdit" />

      <GamePanelsList :panels="panels" />
    </template>

    <GameFormModal
      :visible="modalOpen"
      :target="modalTarget"
      :default-category="defaultCategory"
      @close="closeModal"
    />
  </div>
</template>

<style scoped>
.games-page { padding: 4px 0; }

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
  gap: 12px;
  flex-wrap: wrap;
}

.header-actions {
  display: flex;
  gap: 10px;
  align-items: center;
}

.category-select {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
  min-width: 200px;
}
</style>
