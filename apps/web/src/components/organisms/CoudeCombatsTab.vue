<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { coudeService } from "@/services/coudeService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import type { CoudeCombat } from "../../types";
import AppButton from "../atoms/AppButton.vue";
import AppTabs from "../molecules/AppTabs.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";

const { formatShortDateTime: fmt } = useFormatDate();
const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

defineExpose({ refresh: () => fetchCombats() });

const statusFilter = ref<string>("active");
const combats = ref<CoudeCombat[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const cancelling = ref<string | null>(null);
const expandedRow = ref<string | null>(null);

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
  { key: "active", label: "⏳ En attente d'acceptation" },
  { key: "betting", label: "🎲 Paris ouverts" },
  { key: "resolving", label: "⚙️ En cours de resolution" },
  { key: "accepted", label: "✅ Termines" },
  { key: "refused", label: "🚫 Refuses" },
  { key: "expired", label: "⏰ Expires" },
  { key: "all", label: "📋 Tous" },
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
      // "En cours" = defender pas encore accepte → pending uniquement.
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

const statsActiveCombats = computed(() =>
  combats.value.filter((c) => c.status === "pending").length,
);
const statsTotalMises = computed(() => combats.value.reduce((s, c) => s + c.mise, 0));
const statsTotalTransferred = computed(() =>
  combats.value.reduce((s, c) => s + (c.coins_transferred ?? 0), 0),
);

watch(selectedGuildId, fetchCombats);
watch(statusFilter, fetchCombats);

onMounted(fetchCombats);
</script>

<template>
  <div class="coude-tab">
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

    <AppTabs
      :model-value="statusFilter"
      :tabs="statusOptions"
      class="filter-tabs"
      @update:model-value="(k) => (statusFilter = k)"
    />

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

.kpi-active .kpi-icon { background: color-mix(in srgb, #f39c12 25%, transparent); }
.kpi-total .kpi-icon { background: color-mix(in srgb, #9b59b6 25%, transparent); }
.kpi-bets .kpi-icon { background: color-mix(in srgb, #f1c40f 25%, transparent); }
.kpi-transferred .kpi-icon { background: color-mix(in srgb, #e67e22 25%, transparent); }

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
.table-header--combats, .row-main--combats {
  grid-template-columns: 50px 2.4fr 1fr 1fr 1fr 1.2fr 1.2fr 40px;
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

.col-icon, .col-chevron { text-align: center; }
.status-emoji { font-size: 1.5rem; }

.match-players { display: flex; align-items: center; gap: 8px; font-size: 0.95rem; }
.player-side { font-weight: 600; }
.player-side.attacker { color: var(--danger); }
.player-side.defender { color: var(--info, #3498db); }
.vs { font-size: 0.72rem; color: var(--text-secondary); font-weight: 700; text-transform: uppercase; }
.match-id {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.7rem;
  color: var(--text-secondary);
  margin-top: 2px;
}

.col-mise { display: flex; align-items: baseline; gap: 6px; }
.bet-value { font-size: 1rem; font-weight: 700; color: var(--accent); }
.bet-unit { font-size: 0.7rem; color: var(--text-secondary); }
.rolls { font-family: "JetBrains Mono", monospace; font-size: 0.9rem; }
.transfer-value { color: var(--success); font-weight: 600; }
.col-date { font-size: 0.82rem; color: var(--text-secondary); }

.positive { color: var(--success); font-weight: 600; }
.muted { color: var(--text-secondary); }
.chaos { color: #9b59b6; font-weight: 600; }

.status-badge {
  display: inline-block;
  padding: 4px 10px;
  border-radius: 12px;
  font-size: 0.72rem; font-weight: 600;
  text-transform: uppercase;
}
.status-warning { background: color-mix(in srgb, var(--warning) 20%, transparent); color: var(--warning); }
.status-success { background: color-mix(in srgb, var(--success) 20%, transparent); color: var(--success); }
.status-danger { background: color-mix(in srgb, var(--danger) 20%, transparent); color: var(--danger); }
.status-info { background: color-mix(in srgb, var(--info, #3498db) 20%, transparent); color: var(--info, #3498db); }

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
.actions-block { display: flex; flex-direction: column; gap: 10px; }
.actions-block button { width: 100%; }
.result-message {
  padding: 10px;
  background: color-mix(in srgb, var(--accent) 8%, transparent);
  border-radius: 8px;
  font-size: 0.85rem; font-style: italic;
}
.muted-text {
  margin: 0; padding: 12px;
  background: color-mix(in srgb, var(--text-secondary) 10%, transparent);
  border-radius: 8px;
  text-align: center; font-size: 0.85rem; color: var(--text-secondary);
}

@media (max-width: 1200px) {
  .table-header--combats, .row-main--combats {
    grid-template-columns: 40px 2fr 1fr 1.2fr 1.2fr 30px;
  }
  .col-rolls, .col-transfer, .col-date { display: none; }
}
@media (max-width: 768px) {
  .table-header--combats, .row-main--combats {
    grid-template-columns: 36px 2fr 1fr 30px;
  }
  .col-status { display: none; }
}
</style>
