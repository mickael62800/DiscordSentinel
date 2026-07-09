<script setup lang="ts">
import { ref, computed } from "vue";
import { errMsg } from "@/utils/errMsg";
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
import ChannelSelect from "@/components/atoms/ChannelSelect.vue";
import { gamesService } from "@/services/gamesService";
import { useToast } from "@/composables/useToast";

const { selectedGuildId } = useGuildSelector();
const { games, panels, categories, loading } = useGames();
const { success: toastOk, error: toastErr } = useToast();

const selectedCategory = ref<string>("__all__");

// Deploiement du panneau (bouton -> event Redis -> bot poste/rafraichit).
const deployChannelId = ref<string>("");
const deploying = ref(false);

const canDeploy = computed(
  () =>
    !!selectedGuildId.value &&
    selectedCategory.value !== "__all__" &&
    !!deployChannelId.value,
);

async function onDeploy() {
  if (!selectedGuildId.value || !canDeploy.value) return;
  const category = selectedCategory.value === "__none__" ? null : selectedCategory.value;
  deploying.value = true;
  try {
    await gamesService.deployPanel(selectedGuildId.value, {
      category,
      channel_id: deployChannelId.value,
    });
    toastOk("Panneau envoye au bot — il apparait dans le salon dans un instant.");
  } catch (e) {
    toastErr(`Echec deploiement : ${errMsg(e)}`);
  } finally {
    deploying.value = false;
  }
}

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

      <div class="deploy-card card">
        <h3>Deployer le panneau sur Discord</h3>
        <p class="deploy-hint">
          Choisis une categorie (menu en haut) et le salon ci-dessous, puis deploie.
          Si un panneau existe deja pour cette categorie, il est rafraichi a sa place.
        </p>
        <div class="deploy-row">
          <ChannelSelect v-model="deployChannelId" :guild-id="selectedGuildId ?? null" />
          <AppButton variant="primary" :disabled="!canDeploy || deploying" @click="onDeploy">
            {{ deploying ? "Envoi..." : "Deployer / Rafraichir" }}
          </AppButton>
        </div>
        <p v-if="selectedCategory === '__all__'" class="deploy-warn">
          Selectionne une categorie precise (ou « Sans categorie ») en haut pour pouvoir deployer.
        </p>
      </div>

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
.deploy-card {
  margin-top: 1.5rem;
  padding: 1rem 1.25rem;
}
.deploy-card h3 {
  margin: 0 0 0.25rem;
}
.deploy-hint {
  margin: 0 0 0.75rem;
  opacity: 0.8;
  font-size: 0.9rem;
}
.deploy-row {
  display: flex;
  gap: 0.75rem;
  align-items: center;
  flex-wrap: wrap;
}
.deploy-warn {
  margin: 0.5rem 0 0;
  color: var(--color-warning, #e0a800);
  font-size: 0.85rem;
}
</style>
