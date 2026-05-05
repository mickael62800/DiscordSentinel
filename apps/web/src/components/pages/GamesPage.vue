<script setup lang="ts">
import { ref, computed } from "vue";
import type { Game } from "@/services/gamesService";
import { useGames } from "@/composables/useGames";
import { useGuildSelector } from "@/composables/useGuildSelector";
import AppButton from "@/components/atoms/AppButton.vue";
import EmptyState from "@/components/atoms/EmptyState.vue";
import LoadingState from "@/components/atoms/LoadingState.vue";
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
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
  <AdminPageShell title="Gestion des jeux">
    <template #actions>
      <select
        v-model="selectedCategory"
        class="category-select app-input-base"
        :disabled="!selectedGuildId"
      >
        <option value="__all__">Toutes les categories</option>
        <option value="__none__">(Sans categorie)</option>
        <option v-for="c in categories" :key="c" :value="c">{{ c }}</option>
      </select>
      <AppButton variant="primary" :disabled="!selectedGuildId" @click="openCreate">
        + Ajouter un jeu
      </AppButton>
    </template>

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
  </AdminPageShell>
</template>

<style scoped>
.category-select {
  min-width: 200px;
}
</style>
