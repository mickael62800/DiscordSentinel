<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { coudeService } from "@/services/coudeService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import type { CoudeCombat, CoudePlayer } from "../../types";
import AppButton from "../atoms/AppButton.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";

const { formatShortDateTime: fmt } = useFormatDate();
const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

const activeTab = ref<"combats" | "players">("combats");
// Default : "active" = pending + betting (les combats reellement en cours).
// Les autres statuts DB : accepted (termine), betting (paris ouverts),
// pending (en attente d'acceptation), refused, expired.
const statusFilter = ref<string>("active");
const combats = ref<CoudeCombat[]>([]);
const players = ref<CoudePlayer[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const cancelling = ref<string | null>(null);
const purging = ref(false);

async function handlePurgeAll() {
  if (!selectedGuildId.value) return;
  const ok1 = await confirm({
    title: "Reset total Coup de Coude",
    message:
      "Supprimer DEFINITIVEMENT toutes les donnees Coup de Coude (joueurs, combats, paris, assurances, inventaire, primes) pour cette guild ?",
  });
  if (!ok1) return;
  const ok2 = await confirm({
    title: "Confirmation finale",
    message: "Cette action est IRREVERSIBLE. Tout sera remis a zero.",
  });
  if (!ok2) return;
  purging.value = true;
  try {
    const res = await coudeService.purgeAll(selectedGuildId.value);
    const total = Object.values(res).reduce((a, b) => a + b, 0);
    success(`${total} ligne(s) supprimee(s).`);
    await Promise.all([fetchCombats(), fetchPlayers()]);
  } catch (e) {
    toastError(String(e));
  } finally {
    purging.value = false;
  }
}
const expandedRow = ref<string | null>(null);
const sortKey = ref<"wins" | "winrate" | "level" | "chaos" | "stolen">("wins");

// Etats reels du backend (coude_combats.status) :
//  - pending   : cree, defenseur pas encore repondu
//  - betting   : defenseur a accepte, phase paris ouverte (set_betting)
//  - resolving : worker en train de resoudre (transient)
//  - accepted  : combat termine (naming legacy, c'est la valeur finale ecrite
//                par le worker a la resolution)
//  - refused   : defenseur a refuse
//  - expired   : TTL depasse, ou annule manuellement
const statusLabels: Record<string, string> = {
  pending: "En attente",
  betting: "Paris ouverts",
  resolving: "Resolution…",
  accepted: "Termine",
  refused: "Refuse",
  expired: "Expire",
  active: "En attente",
  all: "Toutes",
};

const statusIcons: Record<string, string> = {
  pending: "⏳",
  betting: "🎲",
  resolving: "⚙️",
  accepted: "✅",
  refused: "🚫",
  expired: "⏰",
};

const statusOptions = [
  { value: "active", label: "⏳ En attente d'acceptation" },
  { value: "betting", label: "🎲 Paris ouverts" },
  { value: "resolving", label: "⚙️ En cours de resolution" },
  { value: "accepted", label: "✅ Termines" },
  { value: "refused", label: "🚫 Refuses" },
  { value: "expired", label: "⏰ Expires" },
  { value: "all", label: "📋 Tous" },
];

function toggleRow(id: string) {
  expandedRow.value = expandedRow.value === id ? null : id;
}

async function fetchCombats() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  error.value = null;
  try {
    if (statusFilter.value === "active") {
      // "En cours" = defender n'a pas encore accepte → pending uniquement.
      // Les autres statuts transients (accepted, betting) sont brefs et vont
      // rapidement vers resolved, donc ils ne polluent pas "En cours".
      combats.value = await coudeService.getCombats(selectedGuildId.value, "pending");
    } else {
      const status = statusFilter.value === "all" ? null : statusFilter.value;
      combats.value = await coudeService.getCombats(selectedGuildId.value, status);
    }
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
    players.value = await coudeService.getPlayers(selectedGuildId.value);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function cancelCombat(id: string) {
  const ok = await confirm({
    title: "Annuler le combat",
    message: "Annuler ce combat ? Les mises ne seront pas remboursees.",
  });
  if (!ok) return;
  cancelling.value = id;
  try {
    await coudeService.cancelCombat(id);
    success("Combat annule");
    await fetchCombats();
  } catch (e) {
    toastError(String(e));
  } finally {
    cancelling.value = null;
  }
}

function statusClass(status: string): string {
  switch (status) {
    case "pending": return "status-warning";
    case "accepted": return "status-success";
    case "refused":
    case "expired":
      return "status-danger";
    default: return "status-info";
  }
}

// Stats combats — "actifs" = pending (defender pas encore accepte).
const statsActiveCombats = computed(() =>
  combats.value.filter((c) => c.status === "pending").length,
);
const statsTotalMises = computed(() => combats.value.reduce((s, c) => s + c.mise, 0));
const statsTotalTransferred = computed(() =>
  combats.value.reduce((s, c) => s + (c.coins_transferred ?? 0), 0),
);

// Stats joueurs
const totalPlayers = computed(() => players.value.length);
const totalCombatsPlayed = computed(() =>
  players.value.reduce((s, p) => s + p.total_wins + p.total_losses + p.total_draws, 0),
);
const avgLevel = computed(() =>
  players.value.length > 0
    ? Math.round(players.value.reduce((s, p) => s + p.level, 0) / players.value.length)
    : 0,
);
const topChaos = computed(() =>
  players.value.length > 0
    ? Math.max(...players.value.map(p => p.cowardice_count))
    : 0,
);

function winRateNum(p: CoudePlayer): number {
  const total = p.total_wins + p.total_losses + p.total_draws;
  if (total === 0) return 0;
  return (p.total_wins / total) * 100;
}

function winRateLabel(p: CoudePlayer): string {
  const total = p.total_wins + p.total_losses + p.total_draws;
  if (total === 0) return "—";
  return `${Math.round(winRateNum(p))}%`;
}

const sortedPlayers = computed(() => {
  const arr = [...players.value];
  switch (sortKey.value) {
    case "wins":
      return arr.sort((a, b) => b.total_wins - a.total_wins);
    case "winrate":
      return arr.sort((a, b) => winRateNum(b) - winRateNum(a));
    case "level":
      return arr.sort((a, b) => b.level - a.level || b.xp - a.xp);
    case "chaos":
      return arr.sort((a, b) => b.cowardice_count - a.cowardice_count);
    case "stolen":
      return arr.sort((a, b) => b.total_stolen - a.total_stolen);
    default:
      return arr;
  }
});

watch(selectedGuildId, () => {
  if (activeTab.value === "combats") fetchCombats();
  else fetchPlayers();
});

watch(activeTab, (tab) => {
  expandedRow.value = null;
  if (tab === "combats") fetchCombats();
  else fetchPlayers();
});

watch(statusFilter, () => fetchCombats());

onMounted(() => fetchCombats());
</script>

<template>
  <div class="coude-page">
    <!-- Hero header -->
    <header class="hero">
      <div class="hero-text">
        <h1 class="hero-title">
          <span class="hero-icon">⚔️</span>
          Coup de Coude
        </h1>
        <p class="hero-subtitle">
          Administration du jeu — suivi des combats et statistiques des joueurs.
          Les coins sont geres via la page <strong>Wallet</strong>.
        </p>
      </div>
      <div class="hero-actions">
        <AppButton
          variant="secondary"
          @click="activeTab === 'combats' ? fetchCombats() : fetchPlayers()"
          :disabled="loading"
        >
          ↻ Rafraichir
        </AppButton>
        <button
          class="danger-btn"
          :disabled="purging"
          @click="handlePurgeAll"
          title="Supprime DEFINITIVEMENT toutes les donnees coude de cette guild"
        >
          {{ purging ? "Purge…" : "🗑 Reset total" }}
        </button>
      </div>
    </header>

    <!-- Tab switcher -->
    <div class="main-tabs">
      <button
        :class="['main-tab', { active: activeTab === 'combats' }]"
        @click="activeTab = 'combats'"
      >
        ⚔️ Combats
      </button>
      <button
        :class="['main-tab', { active: activeTab === 'players' }]"
        @click="activeTab = 'players'"
      >
        📊 Stats joueurs
      </button>
    </div>

    <!-- ═══════════════════════════════════════════════
         TAB COMBATS
         ═══════════════════════════════════════════════ -->
    <template v-if="activeTab === 'combats'">
      <!-- KPI cards combats -->
      <div class="kpi-grid">
        <div class="kpi-card kpi-active">
          <div class="kpi-icon">⏳</div>
          <div class="kpi-content">
            <span class="kpi-label">En attente</span>
            <strong class="kpi-value">{{ statsActiveCombats }}</strong>
          </div>
        </div>
        <div class="kpi-card kpi-total">
          <div class="kpi-icon">⚔️</div>
          <div class="kpi-content">
            <span class="kpi-label">Total combats</span>
            <strong class="kpi-value">{{ combats.length }}</strong>
          </div>
        </div>
        <div class="kpi-card kpi-bets">
          <div class="kpi-icon">💰</div>
          <div class="kpi-content">
            <span class="kpi-label">Mises cumulees</span>
            <strong class="kpi-value">{{ statsTotalMises.toLocaleString() }}</strong>
          </div>
        </div>
        <div class="kpi-card kpi-transferred">
          <div class="kpi-icon">💸</div>
          <div class="kpi-content">
            <span class="kpi-label">Coins transferes</span>
            <strong class="kpi-value">{{ statsTotalTransferred.toLocaleString() }}</strong>
          </div>
        </div>
      </div>

      <!-- Filter tabs -->
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

      <LoadingState v-if="loading" />
      <ErrorState v-else-if="error" :message="error" @retry="fetchCombats" />
      <EmptyState v-else-if="combats.length === 0" message="Aucun combat pour ce filtre" />

      <div v-else class="data-table">
        <div class="table-header table-header--combats">
          <div class="col-icon"></div>
          <div class="col-match">Combat</div>
          <div class="col-mise">Mise</div>
          <div class="col-rolls">Rolls</div>
          <div class="col-transfer">Transfert</div>
          <div class="col-status">Statut</div>
          <div class="col-date">Date</div>
          <div class="col-chevron"></div>
        </div>

        <div
          v-for="c in combats"
          :key="c.id"
          class="table-row"
          :class="{ expanded: expandedRow === c.id }"
        >
          <div class="row-main row-main--combats" @click="toggleRow(c.id)">
            <div class="col-icon">
              <span class="status-emoji">{{ statusIcons[c.status] ?? '❓' }}</span>
            </div>
            <div class="col-match">
              <div class="match-players">
                <span class="player-side attacker">{{ c.attacker_name }}</span>
                <span class="vs">vs</span>
                <span class="player-side defender">{{ c.defender_name }}</span>
              </div>
              <div class="match-id">{{ c.id }}</div>
            </div>
            <div class="col-mise">
              <span class="bet-value">{{ c.mise.toLocaleString() }}</span>
              <span class="bet-unit">coins</span>
            </div>
            <div class="col-rolls">
              <span v-if="c.attacker_roll != null" class="rolls">
                {{ c.attacker_roll }} <span class="muted">vs</span> {{ c.defender_roll }}
              </span>
              <span v-else class="muted">—</span>
            </div>
            <div class="col-transfer">
              <span v-if="c.coins_transferred" class="transfer-value">
                {{ c.coins_transferred.toLocaleString() }}
              </span>
              <span v-else class="muted">—</span>
            </div>
            <div class="col-status">
              <span class="status-badge" :class="statusClass(c.status)">
                {{ statusLabels[c.status] ?? c.status }}
              </span>
            </div>
            <div class="col-date">{{ fmt(c.created_at) }}</div>
            <div class="col-chevron">
              <span class="chevron">{{ expandedRow === c.id ? '▼' : '▶' }}</span>
            </div>
          </div>

          <div v-if="expandedRow === c.id" class="row-actions">
            <div class="detail-grid">
              <div class="detail-block">
                <h4>⚔️ Participants</h4>
                <div class="detail-row">
                  <span class="detail-label">Attaquant</span>
                  <span>{{ c.attacker_name }}</span>
                </div>
                <div class="detail-row">
                  <span class="detail-label">Defenseur</span>
                  <span>{{ c.defender_name }}</span>
                </div>
                <div class="detail-row" v-if="c.winner_id">
                  <span class="detail-label">Vainqueur</span>
                  <code>{{ c.winner_id }}</code>
                </div>
              </div>

              <div class="detail-block">
                <h4>🎲 Resolution</h4>
                <div class="detail-row" v-if="c.attacker_roll != null">
                  <span class="detail-label">Roll attaquant</span>
                  <strong>{{ c.attacker_roll }}</strong>
                </div>
                <div class="detail-row" v-if="c.defender_roll != null">
                  <span class="detail-label">Roll defenseur</span>
                  <strong>{{ c.defender_roll }}</strong>
                </div>
                <div class="detail-row" v-if="c.chaos_event">
                  <span class="detail-label">Chaos</span>
                  <span class="chaos">{{ c.chaos_event }}</span>
                </div>
                <div class="detail-row" v-if="c.special_attack">
                  <span class="detail-label">Attaque speciale</span>
                  <span>{{ c.special_attack }}</span>
                </div>
                <div class="detail-row" v-if="c.defender_special">
                  <span class="detail-label">Defense speciale</span>
                  <span>{{ c.defender_special }}</span>
                </div>
                <div class="detail-row" v-if="c.coins_transferred">
                  <span class="detail-label">Coins transferes</span>
                  <strong class="positive">+{{ c.coins_transferred }}</strong>
                </div>
              </div>

              <div class="detail-block">
                <h4>⏱️ Timeline</h4>
                <div class="detail-row">
                  <span class="detail-label">Creation</span>
                  <span>{{ fmt(c.created_at) }}</span>
                </div>
                <div class="detail-row" v-if="c.resolved_at">
                  <span class="detail-label">Resolution</span>
                  <span>{{ fmt(c.resolved_at) }}</span>
                </div>
              </div>

              <div class="detail-block actions-block">
                <h4>⚡ Actions</h4>
                <div v-if="c.result_message" class="result-message">
                  {{ c.result_message }}
                </div>
                <AppButton
                  v-if="c.status === 'pending'"
                  variant="danger"
                  :disabled="cancelling === c.id"
                  @click.stop="cancelCombat(c.id)"
                >
                  {{ cancelling === c.id ? '⌛ Annulation...' : '🚫 Annuler ce combat' }}
                </AppButton>
                <p v-else class="muted-text">
                  Combat deja resolu — aucune action admin possible.
                </p>
              </div>
            </div>
          </div>
        </div>
      </div>
    </template>

    <!-- ═══════════════════════════════════════════════
         TAB PLAYERS (stats only)
         ═══════════════════════════════════════════════ -->
    <template v-else>
      <!-- KPI cards joueurs -->
      <div class="kpi-grid">
        <div class="kpi-card kpi-players">
          <div class="kpi-icon">👥</div>
          <div class="kpi-content">
            <span class="kpi-label">Joueurs actifs</span>
            <strong class="kpi-value">{{ totalPlayers }}</strong>
          </div>
        </div>
        <div class="kpi-card kpi-matches">
          <div class="kpi-icon">⚔️</div>
          <div class="kpi-content">
            <span class="kpi-label">Total combats joues</span>
            <strong class="kpi-value">{{ totalCombatsPlayed.toLocaleString() }}</strong>
          </div>
        </div>
        <div class="kpi-card kpi-level">
          <div class="kpi-icon">📈</div>
          <div class="kpi-content">
            <span class="kpi-label">Niveau moyen</span>
            <strong class="kpi-value">{{ avgLevel }}</strong>
          </div>
        </div>
        <div class="kpi-card kpi-chaos">
          <div class="kpi-icon">🐔</div>
          <div class="kpi-content">
            <span class="kpi-label">Max lachetes</span>
            <strong class="kpi-value">{{ topChaos }}</strong>
          </div>
        </div>
      </div>

      <!-- Sort tabs -->
      <div class="tabs">
        <button :class="['tab', { active: sortKey === 'wins' }]" @click="sortKey = 'wins'">
          🏆 Top victoires
        </button>
        <button :class="['tab', { active: sortKey === 'winrate' }]" @click="sortKey = 'winrate'">
          📊 Meilleur winrate
        </button>
        <button :class="['tab', { active: sortKey === 'level' }]" @click="sortKey = 'level'">
          ⭐ Plus haut niveau
        </button>
        <button :class="['tab', { active: sortKey === 'stolen' }]" @click="sortKey = 'stolen'">
          🥷 Voleurs
        </button>
        <button :class="['tab', { active: sortKey === 'chaos' }]" @click="sortKey = 'chaos'">
          🐔 Lachetes
        </button>
      </div>

      <LoadingState v-if="loading" />
      <ErrorState v-else-if="error" :message="error" @retry="fetchPlayers" />
      <EmptyState v-else-if="players.length === 0" message="Aucun joueur" />

      <div v-else class="data-table">
        <div class="table-header table-header--players">
          <div class="col-rank">#</div>
          <div class="col-player">Joueur</div>
          <div class="col-level">Niveau</div>
          <div class="col-record">W / L / D</div>
          <div class="col-winrate">Winrate</div>
          <div class="col-specials">Specifique</div>
          <div class="col-chevron"></div>
        </div>

        <div
          v-for="(p, idx) in sortedPlayers"
          :key="p.user_id"
          class="table-row"
          :class="{ expanded: expandedRow === p.user_id }"
        >
          <div class="row-main row-main--players" @click="toggleRow(p.user_id)">
            <div class="col-rank">
              <span class="rank-badge" :class="`rank-${idx < 3 ? idx + 1 : 'default'}`">
                {{ idx + 1 }}
              </span>
            </div>
            <div class="col-player">
              <div class="player-name">{{ p.username }}</div>
              <div class="player-subline">
                <span v-if="p.class" class="class-badge">{{ p.class }}</span>
                <span v-if="p.title" class="title-badge">{{ p.title }}</span>
              </div>
            </div>
            <div class="col-level">
              <div class="level-value">Niv. {{ p.level }}</div>
              <div class="xp-value">{{ p.xp.toLocaleString() }} XP</div>
            </div>
            <div class="col-record">
              <span class="positive">{{ p.total_wins }}</span>
              <span class="sep">/</span>
              <span class="negative">{{ p.total_losses }}</span>
              <span class="sep">/</span>
              <span class="muted">{{ p.total_draws }}</span>
            </div>
            <div class="col-winrate">
              <span class="winrate-value">{{ winRateLabel(p) }}</span>
            </div>
            <div class="col-specials">
              <span v-if="sortKey === 'stolen'" class="metric-hl">
                🥷 {{ p.total_stolen.toLocaleString() }}
              </span>
              <span v-else-if="sortKey === 'chaos'" class="metric-hl">
                🐔 {{ p.cowardice_count }}
              </span>
              <span v-else class="casino-stats">
                🎰 {{ p.casino_wins }}W / {{ p.casino_losses }}L
              </span>
            </div>
            <div class="col-chevron">
              <span class="chevron">{{ expandedRow === p.user_id ? '▼' : '▶' }}</span>
            </div>
          </div>

          <div v-if="expandedRow === p.user_id" class="row-actions">
            <div class="detail-grid">
              <div class="detail-block">
                <h4>🆔 Identite</h4>
                <div class="detail-row">
                  <span class="detail-label">Username</span>
                  <strong>{{ p.username }}</strong>
                </div>
                <div class="detail-row">
                  <span class="detail-label">User ID</span>
                  <code>{{ p.user_id }}</code>
                </div>
                <div class="detail-row" v-if="p.class">
                  <span class="detail-label">Classe</span>
                  <span>{{ p.class }}</span>
                </div>
                <div class="detail-row" v-if="p.title">
                  <span class="detail-label">Titre</span>
                  <span>{{ p.title }}</span>
                </div>
              </div>

              <div class="detail-block">
                <h4>⚔️ Combats</h4>
                <div class="detail-row">
                  <span class="detail-label">Victoires</span>
                  <strong class="positive">{{ p.total_wins }}</strong>
                </div>
                <div class="detail-row">
                  <span class="detail-label">Defaites</span>
                  <strong class="negative">{{ p.total_losses }}</strong>
                </div>
                <div class="detail-row">
                  <span class="detail-label">Egalites</span>
                  <strong class="muted">{{ p.total_draws }}</strong>
                </div>
                <div class="detail-row">
                  <span class="detail-label">Taux de victoire</span>
                  <strong>{{ winRateLabel(p) }}</strong>
                </div>
              </div>

              <div class="detail-block">
                <h4>💰 Flux de coins</h4>
                <div class="detail-row">
                  <span class="detail-label">Gagnes en combat</span>
                  <strong class="positive">+{{ p.total_earned.toLocaleString() }}</strong>
                </div>
                <div class="detail-row">
                  <span class="detail-label">Perdus en combat</span>
                  <strong class="negative">-{{ p.total_lost.toLocaleString() }}</strong>
                </div>
                <div class="detail-row">
                  <span class="detail-label">Voles</span>
                  <strong class="positive">{{ p.total_stolen.toLocaleString() }}</strong>
                </div>
              </div>

              <div class="detail-block">
                <h4>🎰 Casino & Chaos</h4>
                <div class="detail-row">
                  <span class="detail-label">Victoires casino</span>
                  <strong class="positive">{{ p.casino_wins }}</strong>
                </div>
                <div class="detail-row">
                  <span class="detail-label">Defaites casino</span>
                  <strong class="negative">{{ p.casino_losses }}</strong>
                </div>
                <div class="detail-row">
                  <span class="detail-label">Lachetes</span>
                  <strong>🐔 {{ p.cowardice_count }}</strong>
                </div>
              </div>
            </div>

            <div class="wallet-tip">
              💰 Pour ajuster le solde de ce joueur, rends-toi dans la page <strong>Wallet</strong>.
            </div>
          </div>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
/* ═════════════════════════════════════════════════
   Page
   ═════════════════════════════════════════════════ */
.coude-page {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
  max-width: 1400px;
  margin: 0 auto;
}

/* Hero */
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
  color: var(--text-muted);
  font-size: 0.95rem;
}

/* Main tabs (combats / players) */
.main-tabs {
  display: flex;
  gap: 4px;
  padding: 4px;
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
}

.main-tab {
  flex: 1;
  padding: 12px 20px;
  background: transparent;
  color: var(--text-muted);
  border: none;
  border-radius: 7px;
  font-size: 0.95rem;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.main-tab:hover {
  background: color-mix(in srgb, var(--accent) 5%, transparent);
  color: var(--text);
}

.main-tab.active {
  background: var(--accent);
  color: white;
  box-shadow: 0 2px 8px color-mix(in srgb, var(--accent) 30%, transparent);
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
  color: var(--text-muted);
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

.kpi-active .kpi-icon { background: color-mix(in srgb, #f39c12 25%, transparent); }
.kpi-total .kpi-icon { background: color-mix(in srgb, #9b59b6 25%, transparent); }
.kpi-bets .kpi-icon { background: color-mix(in srgb, #f1c40f 25%, transparent); }
.kpi-transferred .kpi-icon { background: color-mix(in srgb, #e67e22 25%, transparent); }
.kpi-players .kpi-icon { background: color-mix(in srgb, #3498db 25%, transparent); }
.kpi-matches .kpi-icon { background: color-mix(in srgb, #e74c3c 25%, transparent); }
.kpi-level .kpi-icon { background: color-mix(in srgb, #2ecc71 25%, transparent); }
.kpi-chaos .kpi-icon { background: color-mix(in srgb, #95a5a6 25%, transparent); }

/* ═════════════════════════════════════════════════
   Filter tabs
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
  color: var(--text-muted);
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
   Data table (shared)
   ═════════════════════════════════════════════════ */
.data-table {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}

.table-header,
.row-main {
  display: grid;
  gap: 16px;
  align-items: center;
  padding: 14px 20px;
}

.table-header--combats,
.row-main--combats {
  grid-template-columns: 50px 2.4fr 1fr 1fr 1fr 1.2fr 1.2fr 40px;
}

.table-header--players,
.row-main--players {
  grid-template-columns: 50px 2fr 1.2fr 1.2fr 1fr 1.4fr 40px;
}

.table-header {
  background: color-mix(in srgb, var(--accent) 5%, var(--surface));
  border-bottom: 2px solid var(--border);
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-muted);
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
.col-icon, .col-rank, .col-chevron {
  text-align: center;
}

.status-emoji {
  font-size: 1.5rem;
}

.rank-badge {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 32px;
  height: 32px;
  border-radius: 50%;
  font-weight: 700;
  font-size: 0.85rem;
  background: var(--bg);
  color: var(--text-muted);
}

.rank-badge.rank-1 {
  background: linear-gradient(135deg, #f1c40f, #f39c12);
  color: white;
}
.rank-badge.rank-2 {
  background: linear-gradient(135deg, #bdc3c7, #95a5a6);
  color: white;
}
.rank-badge.rank-3 {
  background: linear-gradient(135deg, #cd7f32, #a0522d);
  color: white;
}

/* Match display */
.match-players {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 0.95rem;
}

.player-side {
  font-weight: 600;
}

.player-side.attacker {
  color: var(--danger);
}

.player-side.defender {
  color: var(--info, #3498db);
}

.vs {
  font-size: 0.72rem;
  color: var(--text-muted);
  font-weight: 700;
  text-transform: uppercase;
}

.match-id {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.7rem;
  color: var(--text-muted);
  margin-top: 2px;
}

.col-mise {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.bet-value {
  font-size: 1rem;
  font-weight: 700;
  color: var(--accent);
}

.bet-unit {
  font-size: 0.7rem;
  color: var(--text-muted);
}

.rolls {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.9rem;
}

.transfer-value {
  color: var(--success);
  font-weight: 600;
}

.col-date {
  font-size: 0.82rem;
  color: var(--text-muted);
}

/* Player display */
.player-name {
  font-weight: 600;
  font-size: 0.95rem;
}

.player-subline {
  display: flex;
  gap: 6px;
  margin-top: 2px;
}

.class-badge {
  padding: 2px 8px;
  background: color-mix(in srgb, var(--info, #3498db) 15%, transparent);
  color: var(--info, #3498db);
  border-radius: 10px;
  font-size: 0.7rem;
  font-weight: 600;
  text-transform: capitalize;
}

.title-badge {
  padding: 2px 8px;
  background: color-mix(in srgb, #f39c12 15%, transparent);
  color: #f39c12;
  border-radius: 10px;
  font-size: 0.7rem;
  font-weight: 600;
}

.level-value {
  font-weight: 700;
  font-size: 0.9rem;
}

.xp-value {
  font-size: 0.72rem;
  color: var(--text-muted);
  margin-top: 2px;
}

.col-record {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.95rem;
  font-weight: 600;
}

.col-record .sep {
  color: var(--text-muted);
  margin: 0 4px;
}

.winrate-value {
  font-weight: 700;
  font-size: 1rem;
}

.casino-stats,
.metric-hl {
  font-size: 0.85rem;
  color: var(--text-muted);
}

.metric-hl {
  font-weight: 600;
  color: var(--text);
}

/* Shared colors */
.positive {
  color: var(--success);
  font-weight: 600;
}

.negative {
  color: var(--danger);
  font-weight: 600;
}

.muted {
  color: var(--text-muted);
}

.chaos {
  color: #9b59b6;
  font-weight: 600;
}

/* Status badges (combats) */
.status-badge {
  display: inline-block;
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 0.72rem;
  font-weight: 600;
  text-transform: uppercase;
}

.status-warning {
  background: color-mix(in srgb, var(--warning) 20%, transparent);
  color: var(--warning);
}

.status-success {
  background: color-mix(in srgb, var(--success) 20%, transparent);
  color: var(--success);
}

.status-danger {
  background: color-mix(in srgb, var(--danger) 20%, transparent);
  color: var(--danger);
}

.status-info {
  background: color-mix(in srgb, var(--info, #3498db) 20%, transparent);
  color: var(--info, #3498db);
}

/* Chevron */
.chevron {
  font-size: 0.75rem;
  color: var(--text-muted);
}

.table-row.expanded .chevron {
  color: var(--accent);
}

/* ═════════════════════════════════════════════════
   Expanded row details
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
  color: var(--text-muted);
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
  color: var(--text-muted);
}

.detail-row code {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.75rem;
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  padding: 2px 6px;
  border-radius: 4px;
}

.actions-block {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.actions-block button {
  width: 100%;
}

.result-message {
  padding: 10px;
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  border-radius: 8px;
  font-size: 0.85rem;
  font-style: italic;
}

.muted-text {
  margin: 0;
  padding: 12px;
  background: color-mix(in srgb, var(--text-muted) 10%, transparent);
  border-radius: 8px;
  text-align: center;
  font-size: 0.85rem;
  color: var(--text-muted);
}

.wallet-tip {
  margin-top: 16px;
  padding: 12px 16px;
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 30%, var(--border));
  border-radius: 8px;
  font-size: 0.85rem;
  text-align: center;
}

/* Responsive — breakpoints standards --bp-xl (1200px) et --bp-md (768px) */
@media (max-width: 1200px) {
  .table-header--combats,
  .row-main--combats {
    grid-template-columns: 40px 2fr 1fr 1.2fr 1.2fr 30px;
  }
  .col-rolls,
  .col-transfer,
  .col-date {
    display: none;
  }
  .table-header--players,
  .row-main--players {
    grid-template-columns: 40px 2fr 1fr 1fr 30px;
  }
  .col-specials,
  .col-level {
    display: none;
  }
}

@media (max-width: 768px) {
  .table-header--combats,
  .row-main--combats {
    grid-template-columns: 36px 2fr 1fr 30px;
  }
  .col-status {
    display: none;
  }
  .table-header--players,
  .row-main--players {
    grid-template-columns: 36px 2fr 1fr 30px;
  }
  .col-winrate {
    display: none;
  }
  .tab {
    flex: 1 1 auto;
    min-width: unset;
  }
}
</style>
