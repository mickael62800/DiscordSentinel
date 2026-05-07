<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { coudeService } from "@/services/coudeService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import type { CoudePlayer } from "../../types";
import AppTabs from "../molecules/AppTabs.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";

const { selectedGuildId } = useGuildSelector();

defineExpose({ refresh: () => fetchPlayers() });

const players = ref<CoudePlayer[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const expandedRow = ref<string | null>(null);
const sortKey = ref<"wins" | "winrate" | "level" | "chaos" | "stolen">("wins");

const sortTabs = [
  { key: "wins", label: "🏆 Top victoires" },
  { key: "winrate", label: "📊 Meilleur winrate" },
  { key: "level", label: "⭐ Plus haut niveau" },
  { key: "stolen", label: "🥷 Voleurs" },
  { key: "chaos", label: "🐔 Lachetes" },
];

function toggleRow(id: string) {
  expandedRow.value = expandedRow.value === id ? null : id;
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
    case "wins": return arr.sort((a, b) => b.total_wins - a.total_wins);
    case "winrate": return arr.sort((a, b) => winRateNum(b) - winRateNum(a));
    case "level": return arr.sort((a, b) => b.level - a.level || b.xp - a.xp);
    case "chaos": return arr.sort((a, b) => b.cowardice_count - a.cowardice_count);
    case "stolen": return arr.sort((a, b) => b.total_stolen - a.total_stolen);
    default: return arr;
  }
});

watch(selectedGuildId, fetchPlayers);
onMounted(fetchPlayers);
</script>

<template>
  <div class="coude-tab">
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

    <AppTabs
      :model-value="sortKey"
      :tabs="sortTabs"
      class="filter-tabs"
      @update:model-value="(k) => (sortKey = k as typeof sortKey)"
    />

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
  </div>
</template>

<style scoped>
.coude-tab { display: flex; flex-direction: column; gap: 20px; }

/* KPIs */
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
.kpi-card:hover { transform: translateY(-2px); border-color: var(--accent); }
.kpi-icon {
  font-size: 2rem;
  width: 48px; height: 48px;
  display: flex; align-items: center; justify-content: center;
  border-radius: 10px;
  background: color-mix(in srgb, var(--accent) 15%, transparent);
}
.kpi-content { display: flex; flex-direction: column; gap: 2px; }
.kpi-label {
  font-size: 0.78rem; color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: 0.5px; font-weight: 500;
}
.kpi-value { font-size: 1.75rem; font-weight: 700; line-height: 1; color: var(--text); }

.kpi-players .kpi-icon { background: color-mix(in srgb, #3498db 25%, transparent); }
.kpi-matches .kpi-icon { background: color-mix(in srgb, #e74c3c 25%, transparent); }
.kpi-level .kpi-icon { background: color-mix(in srgb, #2ecc71 25%, transparent); }
.kpi-chaos .kpi-icon { background: color-mix(in srgb, #95a5a6 25%, transparent); }

.filter-tabs { width: 100%; }

/* Data table */
.data-table {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}
.table-header, .row-main {
  display: grid;
  gap: 16px;
  align-items: center;
  padding: 14px 20px;
}
.table-header--players, .row-main--players {
  grid-template-columns: 50px 2fr 1.2fr 1.2fr 1fr 1.4fr 40px;
}
.table-header {
  background: color-mix(in srgb, var(--accent) 5%, var(--surface));
  border-bottom: 2px solid var(--border);
  font-size: 0.72rem; font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: 0.5px;
}
.table-row { border-bottom: 1px solid var(--border); }
.table-row:last-child { border-bottom: none; }
.row-main { cursor: pointer; transition: background-color var(--transition-fast); }
.row-main:hover { background: color-mix(in srgb, var(--accent) 4%, transparent); }
.table-row.expanded .row-main { background: color-mix(in srgb, var(--accent) 8%, transparent); }

.col-rank, .col-chevron { text-align: center; }
.rank-badge {
  display: inline-flex; align-items: center; justify-content: center;
  width: 32px; height: 32px;
  border-radius: 50%;
  font-weight: 700; font-size: 0.85rem;
  background: var(--bg);
  color: var(--text-secondary);
}
.rank-badge.rank-1 { background: linear-gradient(135deg, #f1c40f, #f39c12); color: white; }
.rank-badge.rank-2 { background: linear-gradient(135deg, #bdc3c7, #95a5a6); color: white; }
.rank-badge.rank-3 { background: linear-gradient(135deg, #cd7f32, #a0522d); color: white; }

.player-name { font-weight: 600; font-size: 0.95rem; }
.player-subline { display: flex; gap: 6px; margin-top: 2px; }
.class-badge {
  padding: 2px 8px;
  background: color-mix(in srgb, var(--info, #3498db) 15%, transparent);
  color: var(--info, #3498db);
  border-radius: 10px;
  font-size: 0.7rem; font-weight: 600;
  text-transform: capitalize;
}
.title-badge {
  padding: 2px 8px;
  background: color-mix(in srgb, #f39c12 15%, transparent);
  color: #f39c12;
  border-radius: 10px;
  font-size: 0.7rem; font-weight: 600;
}
.level-value { font-weight: 700; font-size: 0.9rem; }
.xp-value { font-size: 0.72rem; color: var(--text-secondary); margin-top: 2px; }

.col-record {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.95rem; font-weight: 600;
}
.col-record .sep { color: var(--text-secondary); margin: 0 4px; }
.winrate-value { font-weight: 700; font-size: 1rem; }
.casino-stats, .metric-hl { font-size: 0.85rem; color: var(--text-secondary); }
.metric-hl { font-weight: 600; color: var(--text); }

.positive { color: var(--success); font-weight: 600; }
.negative { color: var(--danger); font-weight: 600; }
.muted { color: var(--text-secondary); }

.chevron { font-size: 0.75rem; color: var(--text-secondary); }
.table-row.expanded .chevron { color: var(--accent); }

/* Expanded details */
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
  text-transform: uppercase; letter-spacing: 0.5px;
  font-weight: 600;
}
.detail-row {
  display: flex; justify-content: space-between; align-items: center;
  padding: 6px 0; font-size: 0.85rem;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
}
.detail-row:last-child { border-bottom: none; }
.detail-label { color: var(--text-secondary); }
.detail-row code {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.75rem;
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  padding: 2px 6px;
  border-radius: 4px;
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

@media (max-width: 1200px) {
  .table-header--players, .row-main--players {
    grid-template-columns: 40px 2fr 1fr 1fr 30px;
  }
  .col-specials, .col-level { display: none; }
}
@media (max-width: 768px) {
  .table-header--players, .row-main--players {
    grid-template-columns: 36px 2fr 1fr 30px;
  }
  .col-winrate { display: none; }
}
</style>
