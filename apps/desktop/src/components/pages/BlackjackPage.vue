<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import type { BlackjackGame } from "../../types";
import AppButton from "../atoms/AppButton.vue";
import AppBadge from "../atoms/AppBadge.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";

const { formatShortDateTime: fmt } = useFormatDate();
const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

const statusFilter = ref<string>("in_progress");
const games = ref<BlackjackGame[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const cancelling = ref<string | null>(null);

const statusLabels: Record<string, string> = {
  in_progress: "En cours",
  player_blackjack: "Blackjack !",
  player_bust: "Bust joueur",
  dealer_bust: "Bust dealer",
  player_win: "Victoire",
  dealer_win: "Defaite",
  push: "Egalite",
  cancelled: "Annulee",
  all: "Toutes",
};

const statusOptions = [
  { value: "in_progress", label: "En cours" },
  { value: "player_win", label: "Victoires" },
  { value: "dealer_win", label: "Defaites" },
  { value: "cancelled", label: "Annulees" },
  { value: "all", label: "Toutes" },
];

const filteredGames = computed(() => games.value);

async function fetchGames() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  error.value = null;
  try {
    const status = statusFilter.value === "all" ? null : statusFilter.value;
    games.value = await invoke<BlackjackGame[]>("blackjack_list_games", {
      guildId: selectedGuildId.value,
      status,
    });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function cancelGame(game: BlackjackGame) {
  const ok = await confirm({
    message: `Annuler la partie de ${game.username} (mise ${game.bet} coins) ? La mise sera remboursee.`,
  });
  if (!ok) return;

  cancelling.value = game.id;
  try {
    await invoke("blackjack_cancel_game", { gameId: game.id });
    success("Partie annulee et mise remboursee");
    await fetchGames();
  } catch (e) {
    toastError(String(e));
  } finally {
    cancelling.value = null;
  }
}

function statusVariant(status: string): "info" | "success" | "warning" | "danger" {
  switch (status) {
    case "in_progress": return "warning";
    case "player_blackjack":
    case "player_win":
    case "dealer_bust":
      return "success";
    case "dealer_win":
    case "player_bust":
      return "danger";
    case "cancelled":
      return "info";
    default:
      return "info";
  }
}

function isCancellable(status: string): boolean {
  return status === "in_progress" || status === "waiting";
}

watch(selectedGuildId, () => fetchGames());
watch(statusFilter, () => fetchGames());
onMounted(() => fetchGames());
</script>

<template>
  <div class="page">
    <header class="page-header">
      <div>
        <h1 class="page-title">🎰 Blackjack</h1>
        <p class="page-subtitle">Administration des parties de blackjack — liste et annulation</p>
      </div>
      <AppButton variant="secondary" @click="fetchGames" :disabled="loading">
        Rafraichir
      </AppButton>
    </header>

    <div class="toolbar">
      <label for="status-filter">Filtrer par statut :</label>
      <select id="status-filter" v-model="statusFilter" class="select">
        <option v-for="opt in statusOptions" :key="opt.value" :value="opt.value">
          {{ opt.label }}
        </option>
      </select>
      <span class="count">{{ filteredGames.length }} partie(s)</span>
    </div>

    <LoadingState v-if="loading" />
    <ErrorState v-else-if="error" :message="error" @retry="fetchGames" />
    <EmptyState v-else-if="filteredGames.length === 0" message="Aucune partie" />

    <div v-else class="games-list">
      <div v-for="game in filteredGames" :key="game.id" class="game-card">
        <div class="game-header">
          <div class="game-player">
            <strong>{{ game.username }}</strong>
            <span class="user-id">{{ game.user_id }}</span>
          </div>
          <AppBadge
            :variant="statusVariant(game.status)"
            :label="statusLabels[game.status] ?? game.status"
          />
        </div>

        <div class="game-body">
          <div class="game-stat">
            <span class="stat-label">Mise</span>
            <strong>{{ game.bet }} coins</strong>
            <small v-if="game.doubled">(doublee)</small>
          </div>
          <div class="game-stat">
            <span class="stat-label">Score joueur</span>
            <strong>{{ game.player_score }}</strong>
          </div>
          <div class="game-stat">
            <span class="stat-label">Score dealer</span>
            <strong>{{ game.dealer_score }}</strong>
          </div>
          <div class="game-stat" v-if="game.payout !== 0">
            <span class="stat-label">Gain</span>
            <strong :class="{ positive: game.payout > 0, negative: game.payout < 0 }">
              {{ game.payout > 0 ? '+' : '' }}{{ game.payout }} coins
            </strong>
          </div>
        </div>

        <div class="game-footer">
          <span class="game-date">Demarree : {{ fmt(game.created_at) }}</span>
          <span v-if="game.finished_at" class="game-date">Fin : {{ fmt(game.finished_at) }}</span>
          <AppButton
            v-if="isCancellable(game.status)"
            variant="danger"
            :disabled="cancelling === game.id"
            @click="cancelGame(game)"
          >
            {{ cancelling === game.id ? "Annulation..." : "Annuler + rembourser" }}
          </AppButton>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page {
  padding: var(--space-md);
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
}

.page-title {
  margin: 0 0 var(--space-xs);
  font-size: 1.5rem;
}

.page-subtitle {
  margin: 0;
  color: var(--text-muted);
  font-size: 0.875rem;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  background: var(--surface);
  padding: var(--space-sm) var(--space-md);
  border-radius: var(--radius-md);
}

.toolbar label {
  font-size: 0.875rem;
  color: var(--text-muted);
}

.select {
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: var(--space-xs) var(--space-sm);
}

.count {
  margin-left: auto;
  color: var(--text-muted);
  font-size: 0.875rem;
}

.games-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.game-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: var(--space-md);
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
}

.game-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.game-player strong {
  display: block;
  font-size: 1rem;
}

.user-id {
  font-size: 0.75rem;
  color: var(--text-muted);
  font-family: monospace;
}

.game-body {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
  gap: var(--space-sm);
}

.game-stat {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.stat-label {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.game-stat strong {
  font-size: 1.1rem;
}

.game-stat small {
  font-size: 0.75rem;
  color: var(--warning);
}

.game-stat .positive {
  color: var(--success);
}

.game-stat .negative {
  color: var(--danger);
}

.game-footer {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  padding-top: var(--space-sm);
  border-top: 1px solid var(--border);
  font-size: 0.8rem;
  color: var(--text-muted);
}

.game-footer button {
  margin-left: auto;
}

.game-date {
  font-size: 0.8rem;
}
</style>
