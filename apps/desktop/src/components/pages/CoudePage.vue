<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import type { CoudeCombat, CoudePlayer } from "../../types";
import AppButton from "../atoms/AppButton.vue";
import AppBadge from "../atoms/AppBadge.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";

const { formatShortDateTime: fmt } = useFormatDate();
const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();

const activeTab = ref<"combats" | "players">("combats");
const statusFilter = ref<string>("pending");
const combats = ref<CoudeCombat[]>([]);
const players = ref<CoudePlayer[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const cancelling = ref(false);

async function fetchCombats() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  error.value = null;
  try {
    combats.value = await invoke<CoudeCombat[]>("get_coude_combats", {
      guildId: selectedGuildId.value,
      status: statusFilter.value === "all" ? null : statusFilter.value,
    });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function fetchPlayers() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  error.value = null;
  try {
    players.value = await invoke<CoudePlayer[]>("get_coude_players", {
      guildId: selectedGuildId.value,
    });
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function cancelCombat(id: string) {
  const ok = await confirm({ message: "Annuler ce combat ? Les mises ne seront pas remboursees." });
  if (!ok) return;
  cancelling.value = true;
  try {
    await invoke("cancel_coude_combat", { combatId: id });
    await fetchCombats();
  } catch (e) {
    error.value = String(e);
  } finally {
    cancelling.value = false;
  }
}

function statusVariant(status: string): "info" | "success" | "warning" | "danger" {
  switch (status) {
    case "pending": return "warning";
    case "accepted": return "success";
    case "refused": return "info";
    case "expired": return "danger";
    default: return "info";
  }
}

const statusLabel: Record<string, string> = {
  pending: "En attente",
  accepted: "Termine",
  refused: "Refuse",
  expired: "Expire",
};

const winRate = (p: CoudePlayer) => {
  const total = p.total_wins + p.total_losses + p.total_draws;
  if (total === 0) return "0%";
  return `${Math.round((p.total_wins / total) * 100)}%`;
};

watch(selectedGuildId, () => {
  if (activeTab.value === "combats") fetchCombats();
  else fetchPlayers();
});

watch(activeTab, (tab) => {
  if (tab === "combats") fetchCombats();
  else fetchPlayers();
});

watch(statusFilter, () => fetchCombats());

onMounted(() => fetchCombats());
</script>

<template>
  <div class="page">
    <div class="page-header">
      <h1>Coup de Coude</h1>
      <p class="page-subtitle">Gestion du jeu — combats et joueurs</p>
    </div>

    <!-- Tabs -->
    <div class="tabs">
      <button :class="['tab', { active: activeTab === 'combats' }]" @click="activeTab = 'combats'">
        Combats
      </button>
      <button :class="['tab', { active: activeTab === 'players' }]" @click="activeTab = 'players'">
        Joueurs
      </button>
    </div>

    <!-- ===== Combats ===== -->
    <div v-if="activeTab === 'combats'" class="tab-content">
      <div class="filters">
        <select v-model="statusFilter" class="filter-select">
          <option value="all">Tous les statuts</option>
          <option value="pending">En attente</option>
          <option value="accepted">Termines</option>
          <option value="refused">Refuses</option>
          <option value="expired">Expires</option>
        </select>
      </div>

      <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchCombats" />
      <LoadingState v-else-if="loading" />
      <EmptyState v-else-if="combats.length === 0" message="Aucun combat" />

      <div v-else class="combat-list">
        <div v-for="c in combats" :key="c.id" class="combat-card">
          <div class="combat-header">
            <div class="combat-players">
              <span class="player-name">{{ c.attacker_name }}</span>
              <span class="vs">VS</span>
              <span class="player-name">{{ c.defender_name }}</span>
            </div>
            <AppBadge :label="statusLabel[c.status] || c.status" :variant="statusVariant(c.status)" />
          </div>

          <div class="combat-details">
            <div class="detail-item">
              <span class="detail-label">Mise</span>
              <span class="detail-value">{{ c.mise }} coins</span>
            </div>
            <div v-if="c.attacker_roll != null" class="detail-item">
              <span class="detail-label">Rolls</span>
              <span class="detail-value">{{ c.attacker_roll }} vs {{ c.defender_roll }}</span>
            </div>
            <div v-if="c.coins_transferred" class="detail-item">
              <span class="detail-label">Transfert</span>
              <span class="detail-value">{{ c.coins_transferred }} coins</span>
            </div>
            <div v-if="c.chaos_event" class="detail-item">
              <span class="detail-label">Chaos</span>
              <span class="detail-value chaos">{{ c.chaos_event }}</span>
            </div>
            <div v-if="c.special_attack" class="detail-item">
              <span class="detail-label">Special</span>
              <span class="detail-value">{{ c.special_attack }}</span>
            </div>
            <div class="detail-item">
              <span class="detail-label">Date</span>
              <span class="detail-value mono">{{ fmt(c.created_at) }}</span>
            </div>
          </div>

          <div v-if="c.result_message" class="combat-result">
            {{ c.result_message }}
          </div>

          <div v-if="c.status === 'pending'" class="combat-actions">
            <AppButton variant="secondary" size="small" :disabled="cancelling" @click="cancelCombat(c.id)">
              Annuler ce combat
            </AppButton>
          </div>
        </div>
      </div>
    </div>

    <!-- ===== Joueurs ===== -->
    <div v-if="activeTab === 'players'" class="tab-content">
      <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchPlayers" />
      <LoadingState v-else-if="loading" />
      <EmptyState v-else-if="players.length === 0" message="Aucun joueur" />

      <div v-else class="players-table">
        <table>
          <thead>
            <tr>
              <th>Joueur</th>
              <th>Niveau</th>
              <th>Classe</th>
              <th>Coins</th>
              <th>W/L/D</th>
              <th>Win%</th>
              <th>Vol</th>
              <th>Casino</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="p in players" :key="p.user_id">
              <td>
                <div class="player-cell">
                  <span class="player-username">{{ p.username }}</span>
                  <span v-if="p.title" class="player-title">{{ p.title }}</span>
                </div>
              </td>
              <td class="mono">{{ p.level }}</td>
              <td>
                <AppBadge v-if="p.class" :label="p.class" variant="info" />
                <span v-else class="text-muted">-</span>
              </td>
              <td class="mono coins">{{ p.coins.toLocaleString() }}</td>
              <td class="mono">{{ p.total_wins }}/{{ p.total_losses }}/{{ p.total_draws }}</td>
              <td class="mono">{{ winRate(p) }}</td>
              <td class="mono">{{ p.total_stolen.toLocaleString() }}</td>
              <td class="mono">{{ p.casino_wins }}W / {{ p.casino_losses }}L</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page {
  padding: 24px;
}

.page-header {
  margin-bottom: 24px;
}

.page-header h1 {
  font-size: 22px;
  font-weight: 700;
  margin-bottom: 4px;
}

.page-subtitle {
  color: var(--text-secondary);
  font-size: 13px;
}

.tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border);
  margin-bottom: 20px;
}

.tab {
  padding: 10px 20px;
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all 0.2s;
}

.tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.tab:hover:not(.active) {
  color: var(--text-primary);
}

.filters {
  margin-bottom: 16px;
}

.filter-select {
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-primary);
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
}

/* Combat cards */
.combat-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.combat-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 16px;
}

.combat-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.combat-players {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 15px;
}

.player-name {
  font-weight: 600;
}

.vs {
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 700;
}

.combat-details {
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
  margin-bottom: 8px;
}

.detail-item {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.detail-label {
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.detail-value {
  font-size: 13px;
}

.detail-value.chaos {
  color: #f59e0b;
}

.combat-result {
  padding: 8px 12px;
  background: var(--bg-secondary);
  border-radius: 6px;
  font-size: 13px;
  color: var(--text-secondary);
  margin: 8px 0;
}

.combat-actions {
  margin-top: 12px;
  display: flex;
  gap: 8px;
}

/* Players table */
.players-table {
  overflow-x: auto;
}

.players-table table {
  width: 100%;
  border-collapse: collapse;
}

.players-table th {
  text-align: left;
  padding: 10px 12px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
}

.players-table td {
  padding: 10px 12px;
  font-size: 13px;
  border-bottom: 1px solid var(--border);
}

.players-table tr:hover {
  background: var(--bg-secondary);
}

.player-cell {
  display: flex;
  flex-direction: column;
}

.player-username {
  font-weight: 600;
}

.player-title {
  font-size: 11px;
  color: var(--text-secondary);
}

.mono {
  font-family: monospace;
}

.coins {
  color: #f59e0b;
  font-weight: 600;
}

.text-muted {
  color: var(--text-secondary);
}
</style>
