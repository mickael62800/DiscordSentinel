<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { blackjackService, type BlackjackTable, type BlackjackTablePlayer } from "@/services/blackjackService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import type { BlackjackGame } from "../../types";
import AppButton from "../atoms/AppButton.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

const { visible } = useComponentVisibility();
const { formatShortDateTime: fmt } = useFormatDate();
const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

const statusFilter = ref<string>("playing");
const games = ref<BlackjackGame[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const cancelling = ref<string | null>(null);
const expandedRow = ref<string | null>(null);
const purging = ref(false);

async function handlePurgeAll() {
  if (!selectedGuildId.value) return;
  const ok1 = await confirm({
    title: "Reset total Blackjack",
    message: "Supprimer DEFINITIVEMENT toutes les parties et tables blackjack pour cette guild ?",
  });
  if (!ok1) return;
  const ok2 = await confirm({
    title: "Confirmation finale",
    message: "Cette action est IRREVERSIBLE. Aucun remboursement ne sera effectue.",
  });
  if (!ok2) return;
  purging.value = true;
  try {
    const res = await blackjackService.purgeAll(selectedGuildId.value);
    success(`${res.deleted_games} partie(s) et ${res.deleted_tables} table(s) supprimee(s).`);
    await fetchGames();
  } catch (e) {
    toastError(String(e));
  } finally {
    purging.value = false;
  }
}

const statusLabels: Record<string, string> = {
  playing: "En cours",
  waiting: "En attente",
  player_blackjack: "Blackjack !",
  player_bust: "Bust joueur",
  dealer_bust: "Bust dealer",
  player_win: "Victoire",
  dealer_win: "Defaite",
  push: "Egalite",
  cancelled: "Annulee",
  all: "Toutes",
};

const statusIcons: Record<string, string> = {
  playing: "🎮",
  waiting: "⏳",
  player_blackjack: "🎰",
  player_bust: "💥",
  dealer_bust: "💀",
  player_win: "✨",
  dealer_win: "😔",
  push: "🤝",
  cancelled: "🚫",
};

const statusOptions = [
  { value: "playing", label: "🎮 En cours", count: 0 },
  { value: "player_win", label: "✨ Victoires", count: 0 },
  { value: "dealer_win", label: "😔 Defaites", count: 0 },
  { value: "cancelled", label: "🚫 Annulees", count: 0 },
  { value: "all", label: "📋 Toutes", count: 0 },
];

function toggleRow(id: string) {
  expandedRow.value = expandedRow.value === id ? null : id;
}

async function fetchGames() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  error.value = null;
  try {
    const status = statusFilter.value === "all" ? null : statusFilter.value;
    games.value = await blackjackService.listGames(selectedGuildId.value, status);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function cancelGame(game: BlackjackGame) {
  const ok = await confirm({
    title: "Annuler la partie",
    message: `Annuler la partie de ${game.username} (mise ${game.bet} coins) ? La mise sera remboursee sur son wallet.`,
  });
  if (!ok) return;

  cancelling.value = game.id;
  try {
    await blackjackService.cancelGame(game.id);
    success("Partie annulee et mise remboursee");
    await fetchGames();
  } catch (e) {
    toastError(String(e));
  } finally {
    cancelling.value = null;
  }
}

function statusClass(status: string): string {
  switch (status) {
    case "playing":
    case "waiting":
      return "status-warning";
    case "player_blackjack":
    case "player_win":
    case "dealer_bust":
      return "status-success";
    case "dealer_win":
    case "player_bust":
      return "status-danger";
    case "push":
      return "status-info";
    case "cancelled":
      return "status-muted";
    default:
      return "status-info";
  }
}

function isCancellable(status: string): boolean {
  return status === "playing" || status === "waiting";
}

// Stats globales
const statsInProgress = computed(() => games.value.filter(g => g.status === "playing" || g.status === "waiting").length);
const statsTotalBet = computed(() => games.value.reduce((s, g) => s + g.bet, 0));
const statsWinRate = computed(() => {
  const finished = games.value.filter(g => ["player_win", "player_blackjack", "dealer_bust", "dealer_win", "player_bust"].includes(g.status));
  if (finished.length === 0) return 0;
  const wins = finished.filter(g => ["player_win", "player_blackjack", "dealer_bust"].includes(g.status)).length;
  return Math.round((wins / finished.length) * 100);
});

// Tables multijoueur
const tables = ref<BlackjackTable[]>([]);
const tablesLoading = ref(false);
const expandedTable = ref<string | null>(null);
const tablePlayers = ref<Record<string, BlackjackTablePlayer[]>>({});
const closingTable = ref<string | null>(null);

async function fetchTables() {
  if (!selectedGuildId.value) return;
  tablesLoading.value = true;
  try {
    tables.value = await blackjackService.listTables(selectedGuildId.value);
  } catch (e) {
    toastError(String(e));
  } finally {
    tablesLoading.value = false;
  }
}

async function toggleTable(id: string) {
  if (expandedTable.value === id) {
    expandedTable.value = null;
    return;
  }
  expandedTable.value = id;
  if (!tablePlayers.value[id]) {
    try {
      tablePlayers.value[id] = await blackjackService.listTablePlayers(id);
    } catch (e) {
      toastError(String(e));
    }
  }
}

async function closeTable(table: BlackjackTable) {
  const ok = await confirm({
    title: "Fermer la table",
    message: `Fermer la table de ${table.owner_name} (channel ${table.channel_id}) ?`,
  });
  if (!ok) return;
  closingTable.value = table.id;
  try {
    await blackjackService.closeTable(table.id);
    success("Table fermee.");
    await fetchTables();
  } catch (e) {
    toastError(String(e));
  } finally {
    closingTable.value = null;
  }
}

watch(selectedGuildId, () => { fetchGames(); fetchTables(); });
watch(statusFilter, () => fetchGames());
onMounted(() => { fetchGames(); fetchTables(); });
</script>

<template>
  <div class="blackjack-page">
    <!-- Hero header -->
    <header class="hero">
      <div class="hero-text">
        <h1 class="hero-title">
          <span class="hero-icon">🎰</span>
          Blackjack
        </h1>
        <p class="hero-subtitle">
          Administration des parties — surveillance, historique, annulation avec remboursement
        </p>
      </div>
      <div class="hero-actions">
        <AppButton variant="secondary" @click="fetchGames" :disabled="loading">
          ↻ Rafraichir
        </AppButton>
        <button
          v-if="visible('db.purge.blackjack')"
          class="danger-btn"
          :disabled="purging"
          @click="handlePurgeAll"
          title="Supprime DEFINITIVEMENT toutes les parties blackjack de cette guild (owner uniquement)"
        >
          {{ purging ? "Purge…" : "🗑 Reset total" }}
        </button>
      </div>
    </header>

    <!-- KPI cards -->
    <div class="kpi-grid">
      <div class="kpi-card kpi-active">
        <div class="kpi-icon">🎮</div>
        <div class="kpi-content">
          <span class="kpi-label">Parties en cours</span>
          <strong class="kpi-value">{{ statsInProgress }}</strong>
        </div>
      </div>
      <div class="kpi-card kpi-total">
        <div class="kpi-icon">📊</div>
        <div class="kpi-content">
          <span class="kpi-label">Total parties (200 max)</span>
          <strong class="kpi-value">{{ games.length }}</strong>
        </div>
      </div>
      <div class="kpi-card kpi-bets">
        <div class="kpi-icon">💰</div>
        <div class="kpi-content">
          <span class="kpi-label">Total mise</span>
          <strong class="kpi-value">{{ statsTotalBet.toLocaleString() }}</strong>
        </div>
      </div>
      <div class="kpi-card kpi-winrate">
        <div class="kpi-icon">📈</div>
        <div class="kpi-content">
          <span class="kpi-label">Taux de victoire</span>
          <strong class="kpi-value">{{ statsWinRate }}%</strong>
        </div>
      </div>
    </div>

    <!-- Tables multijoueur -->
    <section class="tables-section">
      <div class="section-header">
        <h2>🎲 Tables multijoueur ouvertes</h2>
        <AppButton variant="secondary" @click="fetchTables" :disabled="tablesLoading">↻</AppButton>
      </div>
      <div v-if="tablesLoading" class="muted-text">Chargement…</div>
      <div v-else-if="tables.length === 0" class="muted-text">Aucune table ouverte.</div>
      <div v-else class="tables-list">
        <div v-for="t in tables" :key="t.id" class="table-card" :class="{ expanded: expandedTable === t.id }">
          <div class="table-card-main" @click="toggleTable(t.id)">
            <div class="table-info">
              <strong>{{ t.owner_name }}</strong>
              <span class="muted">channel <code>{{ t.channel_id }}</code></span>
            </div>
            <div class="table-meta">
              <span class="muted">{{ fmt(t.created_at) }}</span>
              <span class="status-badge status-success">{{ t.status }}</span>
              <button
                class="danger-btn"
                :disabled="closingTable === t.id"
                @click.stop="closeTable(t)"
                title="Fermer la table"
              >{{ closingTable === t.id ? "…" : "🚫 Fermer" }}</button>
            </div>
          </div>
          <div v-if="expandedTable === t.id" class="table-card-detail">
            <h4>Joueurs</h4>
            <ul v-if="tablePlayers[t.id]?.length" class="players-list">
              <li v-for="p in tablePlayers[t.id]" :key="p.user_id">
                <strong>{{ p.user_name }}</strong>
                <code>{{ p.user_id }}</code>
                <span class="muted">{{ fmt(p.joined_at) }}</span>
              </li>
            </ul>
            <p v-else class="muted-text">Aucun joueur encore.</p>
            <div class="detail-row"><span class="detail-label">Table ID</span><code>{{ t.id }}</code></div>
            <div class="detail-row"><span class="detail-label">Owner ID</span><code>{{ t.owner_id }}</code></div>
          </div>
        </div>
      </div>
    </section>

    <!-- Status filter tabs -->
    <div class="tabs">
      <button
        v-for="opt in statusOptions"
        :key="opt.value"
        :class="['tab', { active: statusFilter === opt.value }]"
        @click="statusFilter = opt.value"
      >
        {{ opt.label }}
      </button>
    </div>

    <!-- Games table -->
    <LoadingState v-if="loading" />
    <ErrorState v-else-if="error" :message="error" @retry="fetchGames" />
    <EmptyState v-else-if="games.length === 0" message="Aucune partie trouvee pour ce filtre" />

    <div v-else class="games-table">
      <div class="table-header">
        <div class="col-icon"></div>
        <div class="col-player">Joueur</div>
        <div class="col-bet">Mise</div>
        <div class="col-scores">Scores</div>
        <div class="col-payout">Gain</div>
        <div class="col-status">Statut</div>
        <div class="col-date">Date</div>
        <div class="col-chevron"></div>
      </div>

      <div
        v-for="game in games"
        :key="game.id"
        class="table-row"
        :class="{ expanded: expandedRow === game.id }"
      >
        <div class="row-main" @click="toggleRow(game.id)">
          <div class="col-icon">
            <span class="status-emoji">{{ statusIcons[game.status] ?? '❓' }}</span>
          </div>
          <div class="col-player">
            <div class="player-name">{{ game.username }}</div>
            <div class="player-id">{{ game.user_id }}</div>
          </div>
          <div class="col-bet">
            <span class="bet-value">{{ game.bet.toLocaleString() }}</span>
            <span class="bet-unit">coins</span>
            <span v-if="game.doubled" class="doubled-badge">2x</span>
          </div>
          <div class="col-scores">
            <div class="score-line">
              <span class="score-label">J</span>
              <strong class="score-value" :class="{ bust: game.player_score > 21 }">
                {{ game.player_score }}
              </strong>
            </div>
            <div class="score-line">
              <span class="score-label">D</span>
              <strong class="score-value" :class="{ bust: game.dealer_score > 21 }">
                {{ game.dealer_score }}
              </strong>
            </div>
          </div>
          <div class="col-payout">
            <span
              v-if="game.payout !== 0"
              :class="{ positive: game.payout > 0, negative: game.payout < 0 }"
            >
              {{ game.payout > 0 ? '+' : '' }}{{ game.payout.toLocaleString() }}
            </span>
            <span v-else class="muted">—</span>
          </div>
          <div class="col-status">
            <span class="status-badge" :class="statusClass(game.status)">
              {{ statusLabels[game.status] ?? game.status }}
            </span>
          </div>
          <div class="col-date">
            {{ fmt(game.created_at) }}
          </div>
          <div class="col-chevron">
            <span class="chevron">{{ expandedRow === game.id ? '▼' : '▶' }}</span>
          </div>
        </div>

        <!-- Panel d'actions expand -->
        <div v-if="expandedRow === game.id" class="row-actions">
          <div class="detail-grid">
            <div class="detail-block">
              <h4>🆔 Identifiants</h4>
              <div class="detail-row">
                <span class="detail-label">Game ID</span>
                <code>{{ game.id }}</code>
              </div>
              <div class="detail-row">
                <span class="detail-label">Guild</span>
                <code>{{ game.guild_id }}</code>
              </div>
              <div class="detail-row">
                <span class="detail-label">User</span>
                <code>{{ game.user_id }}</code>
              </div>
            </div>

            <div class="detail-block">
              <h4>⏱️ Timeline</h4>
              <div class="detail-row">
                <span class="detail-label">Demarree</span>
                <span>{{ fmt(game.created_at) }}</span>
              </div>
              <div class="detail-row" v-if="game.finished_at">
                <span class="detail-label">Terminee</span>
                <span>{{ fmt(game.finished_at) }}</span>
              </div>
              <div class="detail-row" v-else>
                <span class="detail-label">Terminee</span>
                <span class="muted">En cours...</span>
              </div>
            </div>

            <div class="detail-block">
              <h4>🎲 Detail du jeu</h4>
              <div class="detail-row">
                <span class="detail-label">Score joueur</span>
                <strong :class="{ bust: game.player_score > 21 }">{{ game.player_score }}</strong>
              </div>
              <div class="detail-row">
                <span class="detail-label">Score dealer</span>
                <strong :class="{ bust: game.dealer_score > 21 }">{{ game.dealer_score }}</strong>
              </div>
              <div class="detail-row">
                <span class="detail-label">Double</span>
                <span>{{ game.doubled ? '✓ Oui' : '✗ Non' }}</span>
              </div>
            </div>

            <div class="detail-block actions-block">
              <h4>⚡ Actions admin</h4>
              <AppButton
                v-if="isCancellable(game.status)"
                variant="danger"
                :disabled="cancelling === game.id"
                @click.stop="cancelGame(game)"
              >
                {{ cancelling === game.id ? '⌛ Annulation...' : '🚫 Annuler + rembourser' }}
              </AppButton>
              <p v-else class="muted-text">
                Partie deja terminee — aucune action admin possible.
              </p>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* ═════════════════════════════════════════════════
   Page
   ═════════════════════════════════════════════════ */
.blackjack-page {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  max-width: 1400px;
  margin: 0 auto;
}

/* ═════════════════════════════════════════════════
   Hero
   ═════════════════════════════════════════════════ */
.hero {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--border);
}

.hero-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.danger-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 8px 14px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: all var(--transition-fast);
}

.danger-btn:hover:not(:disabled) {
  background: var(--danger);
  color: white;
}

.danger-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.hero-title {
  display: flex;
  align-items: center;
  gap: 12px;
  margin: 0 0 6px;
  font-size: 2rem;
  font-weight: 700;
}

.hero-icon {
  font-size: 2rem;
}

.hero-subtitle {
  margin: 0;
  color: var(--text-secondary);
  font-size: 0.95rem;
}

/* ═════════════════════════════════════════════════
   KPIs
   ═════════════════════════════════════════════════ */
.kpi-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 16px;
}

.kpi-card {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 18px 20px;
  display: flex;
  align-items: center;
  gap: 16px;
  transition: transform var(--transition-fast), border-color var(--transition-fast);
}

.kpi-card:hover {
  transform: translateY(-2px);
  border-color: var(--accent);
}

.kpi-icon {
  font-size: 2rem;
  width: 48px;
  height: 48px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 10px;
  background: color-mix(in srgb, var(--accent) 15%, transparent);
}

.kpi-content {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.kpi-label {
  font-size: 0.78rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 500;
}

.kpi-value {
  font-size: 1.75rem;
  font-weight: 700;
  line-height: 1;
  color: var(--text);
}

.kpi-active .kpi-icon { background: color-mix(in srgb, #3498db 25%, transparent); }
.kpi-total .kpi-icon { background: color-mix(in srgb, #9b59b6 25%, transparent); }
.kpi-bets .kpi-icon { background: color-mix(in srgb, #f1c40f 25%, transparent); }
.kpi-winrate .kpi-icon { background: color-mix(in srgb, #2ecc71 25%, transparent); }

/* ═════════════════════════════════════════════════
   Tabs
   ═════════════════════════════════════════════════ */
.tabs {
  display: flex;
  gap: 4px;
  padding: 4px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  flex-wrap: wrap;
}

.tab {
  flex: 1;
  min-width: 120px;
  padding: 10px 16px;
  background: transparent;
  color: var(--text-secondary);
  border: none;
  border-radius: 7px;
  font-size: 0.85rem;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.tab:hover {
  background: color-mix(in srgb, var(--accent) 5%, transparent);
  color: var(--text);
}

.tab.active {
  background: var(--accent);
  color: white;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 30%, transparent);
}

/* ═════════════════════════════════════════════════
   Games table
   ═════════════════════════════════════════════════ */
.games-table {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}

.table-header,
.row-main {
  display: grid;
  grid-template-columns: 50px 2fr 1.2fr 1fr 1fr 1.2fr 1.2fr 40px;
  gap: 16px;
  align-items: center;
  padding: 14px 20px;
}

.table-header {
  background: color-mix(in srgb, var(--accent) 5%, var(--surface));
  border-bottom: 2px solid var(--border);
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.table-row {
  border-bottom: 1px solid var(--border);
}

.table-row:last-child {
  border-bottom: none;
}

.row-main {
  cursor: pointer;
  transition: background-color var(--transition-fast);
}

.row-main:hover {
  background: color-mix(in srgb, var(--accent) 4%, transparent);
}

.table-row.expanded .row-main {
  background: color-mix(in srgb, var(--accent) 8%, transparent);
}

/* Columns */
.col-icon {
  text-align: center;
}

.status-emoji {
  font-size: 1.5rem;
}

.player-name {
  font-weight: 600;
  font-size: 0.95rem;
}

.player-id {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.72rem;
  color: var(--text-secondary);
  margin-top: 2px;
}

.col-bet {
  display: flex;
  align-items: baseline;
  gap: 6px;
  flex-wrap: wrap;
}

.bet-value {
  font-size: 1rem;
  font-weight: 700;
  color: var(--accent);
}

.bet-unit {
  font-size: 0.7rem;
  color: var(--text-secondary);
}

.doubled-badge {
  display: inline-block;
  padding: 2px 6px;
  background: var(--warning);
  color: white;
  border-radius: 4px;
  font-size: 0.65rem;
  font-weight: 700;
}

.col-scores {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.score-line {
  display: flex;
  align-items: baseline;
  gap: 6px;
  font-size: 0.85rem;
}

.score-label {
  width: 12px;
  color: var(--text-secondary);
  font-weight: 600;
}

.score-value {
  color: var(--text);
}

.score-value.bust {
  color: var(--danger);
  text-decoration: line-through;
}

.col-payout {
  font-weight: 700;
}

.col-payout .positive {
  color: var(--success);
}

.col-payout .negative {
  color: var(--danger);
}

.col-payout .muted,
.muted,
.muted-text {
  color: var(--text-secondary);
}

/* Status badge */
.status-badge {
  display: inline-block;
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 0.72rem;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.status-badge.status-warning {
  background: color-mix(in srgb, var(--warning) 20%, transparent);
  color: var(--warning);
}

.status-badge.status-success {
  background: color-mix(in srgb, var(--success) 20%, transparent);
  color: var(--success);
}

.status-badge.status-danger {
  background: color-mix(in srgb, var(--danger) 20%, transparent);
  color: var(--danger);
}

.status-badge.status-info {
  background: color-mix(in srgb, var(--info, #3498db) 20%, transparent);
  color: var(--info, #3498db);
}

.status-badge.status-muted {
  background: color-mix(in srgb, var(--text-secondary) 20%, transparent);
  color: var(--text-secondary);
}

.col-date {
  font-size: 0.82rem;
  color: var(--text-secondary);
}

.col-chevron {
  text-align: center;
}

.chevron {
  font-size: 0.75rem;
  color: var(--text-secondary);
}

.table-row.expanded .chevron {
  color: var(--accent);
}

/* ═════════════════════════════════════════════════
   Expanded actions
   ═════════════════════════════════════════════════ */
.row-actions {
  padding: 20px;
  background: color-mix(in srgb, var(--accent) 3%, var(--bg));
  border-top: 1px dashed var(--border);
}

.detail-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
  gap: 16px;
}

.detail-block {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 14px 16px;
}

.detail-block h4 {
  margin: 0 0 12px;
  font-size: 0.78rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
}

.detail-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 0;
  font-size: 0.85rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
}

.detail-row:last-child {
  border-bottom: none;
}

.detail-label {
  color: var(--text-secondary);
}

.detail-row code {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.75rem;
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  padding: 2px 6px;
  border-radius: 4px;
}

.detail-row strong.bust {
  color: var(--danger);
  text-decoration: line-through;
}

.actions-block {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.actions-block button {
  width: 100%;
}

.muted-text {
  margin: 0;
  padding: 12px;
  background: color-mix(in srgb, var(--text-secondary) 10%, transparent);
  border-radius: 8px;
  text-align: center;
  font-size: 0.85rem;
}

/* Tables multijoueur */
.tables-section {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.section-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
}
.section-header h2 {
  margin: 0;
  font-size: 1.1rem;
}
.tables-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.table-card {
  border: 1px solid var(--border);
  border-radius: 8px;
  overflow: hidden;
}
.table-card-main {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  cursor: pointer;
  gap: 12px;
}
.table-card-main:hover {
  background: color-mix(in srgb, var(--accent) 4%, transparent);
}
.table-info { display: flex; flex-direction: column; gap: 2px; }
.table-meta { display: flex; align-items: center; gap: 10px; }
.table-card-detail {
  padding: 12px 16px;
  border-top: 1px dashed var(--border);
  background: color-mix(in srgb, var(--accent) 3%, var(--bg));
}
.table-card-detail h4 { margin: 0 0 8px; font-size: 0.85rem; }
.players-list { list-style: none; padding: 0; margin: 0 0 10px; }
.players-list li {
  display: flex; gap: 10px; align-items: baseline;
  padding: 4px 0;
  font-size: 0.85rem;
}
.players-list code {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.72rem;
  color: var(--accent);
}

/* Responsive — breakpoints standards --bp-xl (1200px) et --bp-md (768px) */
@media (max-width: 1200px) {
  .table-header,
  .row-main {
    grid-template-columns: 40px 2fr 1.2fr 1fr 1.2fr 30px;
  }
  .col-scores,
  .col-date {
    display: none;
  }
}

@media (max-width: 768px) {
  .table-header,
  .row-main {
    grid-template-columns: 36px 2fr 1fr 30px;
  }
  .col-bet {
    display: none;
  }
  .tab {
    flex: 1 1 auto;
    min-width: unset;
  }
  .hero {
    flex-direction: column;
    align-items: flex-start;
    gap: 12px;
  }
  .hero-actions {
    width: 100%;
    flex-wrap: wrap;
  }
  .hero-actions > * {
    flex: 1;
  }
}
</style>
