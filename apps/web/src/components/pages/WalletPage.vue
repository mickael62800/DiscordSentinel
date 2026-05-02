<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { walletService } from "@/services/walletService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import type { Wallet } from "../../types";
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

const wallets = ref<Wallet[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const busy = ref<string | null>(null);
const search = ref("");
const resetAllBalance = ref<number>(100);
const expandedRow = ref<string | null>(null);

const amounts = ref<Record<string, number>>({});
const resetBalances = ref<Record<string, number>>({});

function getAmount(userId: string): number {
  return amounts.value[userId] ?? 100;
}
function setAmount(userId: string, val: number) {
  amounts.value[userId] = val;
}
function getResetBalance(userId: string): number {
  return resetBalances.value[userId] ?? 100;
}
function setResetBalance(userId: string, val: number) {
  resetBalances.value[userId] = val;
}

function toggleRow(userId: string) {
  expandedRow.value = expandedRow.value === userId ? null : userId;
}

const filtered = computed(() => {
  if (!search.value) return wallets.value;
  const q = search.value.toLowerCase();
  return wallets.value.filter(
    (w) => w.username.toLowerCase().includes(q) || w.user_id.includes(q),
  );
});

const totalCoins = computed(() =>
  wallets.value.reduce((sum, w) => sum + w.coins, 0),
);

const richest = computed(() =>
  wallets.value.length > 0 ? Math.max(...wallets.value.map((w) => w.coins)) : 0,
);

async function fetchWallets() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  error.value = null;
  try {
    wallets.value = await walletService.list(selectedGuildId.value);
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

async function credit(wallet: Wallet) {
  if (!selectedGuildId.value) return;
  const amount = getAmount(wallet.user_id);
  if (amount <= 0) {
    toastError("Le montant doit etre positif.");
    return;
  }
  busy.value = wallet.user_id;
  try {
    await walletService.credit(selectedGuildId.value, wallet.user_id, amount, "Credit admin desktop");
    success(`+${amount} coins credites a ${wallet.username}`);
    await fetchWallets();
  } catch (e) {
    toastError(String(e));
  } finally {
    busy.value = null;
  }
}

async function debit(wallet: Wallet) {
  if (!selectedGuildId.value) return;
  const amount = getAmount(wallet.user_id);
  if (amount <= 0) {
    toastError("Le montant doit etre positif.");
    return;
  }
  busy.value = wallet.user_id;
  try {
    await walletService.debit(selectedGuildId.value, wallet.user_id, amount, "Debit admin desktop");
    success(`-${amount} coins debites de ${wallet.username}`);
    await fetchWallets();
  } catch (e) {
    toastError(String(e));
  } finally {
    busy.value = null;
  }
}

async function resetOne(wallet: Wallet) {
  if (!selectedGuildId.value) return;
  const newBalance = getResetBalance(wallet.user_id);
  const ok = await confirm({
    title: "Reset du wallet",
    message: `Reset le wallet de ${wallet.username} a ${newBalance} coins ? Son historique sera efface.`,
  });
  if (!ok) return;

  busy.value = wallet.user_id;
  try {
    await walletService.reset(selectedGuildId.value, wallet.user_id, newBalance);
    success(`Wallet de ${wallet.username} reset a ${newBalance} coins`);
    await fetchWallets();
  } catch (e) {
    toastError(String(e));
  } finally {
    busy.value = null;
  }
}

async function resetAll() {
  if (!selectedGuildId.value) return;
  const ok = await confirm({
    title: "Reset global des wallets",
    message: `Reset TOUS les ${wallets.value.length} wallets a ${resetAllBalance.value} coins ? L'historique complet sera efface. Irreversible.`,
  });
  if (!ok) return;

  loading.value = true;
  try {
    const affected = await walletService.resetAll(selectedGuildId.value, resetAllBalance.value);
    success(`${affected} wallet(s) reset a ${resetAllBalance.value} coins`);
    await fetchWallets();
  } catch (e) {
    toastError(String(e));
  } finally {
    loading.value = false;
  }
}

watch(selectedGuildId, () => fetchWallets());
onMounted(() => fetchWallets());
</script>

<template>
  <div class="wallet-page">
    <!-- Hero header -->
    <header class="hero">
      <div class="hero-text">
        <h1 class="hero-title">
          <span class="hero-icon">💰</span>
          Wallet
        </h1>
        <p class="hero-subtitle">
          Monnaie partagee entre tous les jeux — Blackjack, Coude, Casino
        </p>
      </div>
      <AppButton variant="secondary" @click="fetchWallets" :disabled="loading">
        ↻ Rafraichir
      </AppButton>
    </header>

    <!-- KPI cards -->
    <div class="kpi-grid">
      <div class="kpi-card kpi-players">
        <div class="kpi-icon">👥</div>
        <div class="kpi-content">
          <span class="kpi-label">Joueurs</span>
          <strong class="kpi-value">{{ wallets.length }}</strong>
        </div>
      </div>
      <div class="kpi-card kpi-total">
        <div class="kpi-icon">💎</div>
        <div class="kpi-content">
          <span class="kpi-label">Coins en circulation</span>
          <strong class="kpi-value">{{ totalCoins.toLocaleString() }}</strong>
        </div>
      </div>
      <div class="kpi-card kpi-avg">
        <div class="kpi-icon">📊</div>
        <div class="kpi-content">
          <span class="kpi-label">Moyenne par joueur</span>
          <strong class="kpi-value">
            {{ wallets.length > 0 ? Math.round(totalCoins / wallets.length).toLocaleString() : 0 }}
          </strong>
        </div>
      </div>
      <div class="kpi-card kpi-top">
        <div class="kpi-icon">👑</div>
        <div class="kpi-content">
          <span class="kpi-label">Plus riche</span>
          <strong class="kpi-value">{{ richest.toLocaleString() }}</strong>
        </div>
      </div>
    </div>

    <!-- Danger zone : reset global (owner uniquement) -->
    <div v-if="visible('db.reset.wallets')" class="danger-zone">
      <div class="danger-icon">⚠️</div>
      <div class="danger-info">
        <h3>Reset global</h3>
        <p>Remet tous les wallets au solde choisi et efface l'historique des transactions.</p>
      </div>
      <div class="danger-actions">
        <div class="input-group">
          <label>Nouveau solde</label>
          <input
            type="number"
            v-model.number="resetAllBalance"
            min="0"
            class="input input-sm"
          />
        </div>
        <AppButton variant="danger" @click="resetAll" :disabled="loading || wallets.length === 0">
          🔥 Reset tout
        </AppButton>
      </div>
    </div>

    <!-- Search bar -->
    <div class="search-bar">
      <div class="search-input">
        <span class="search-icon">🔍</span>
        <input
          type="text"
          v-model="search"
          placeholder="Rechercher par nom ou user_id..."
          class="input"
        />
      </div>
      <span class="count">{{ filtered.length }} wallet(s)</span>
    </div>

    <!-- Wallets table -->
    <LoadingState v-if="loading" />
    <ErrorState v-else-if="error" :message="error" @retry="fetchWallets" />
    <EmptyState v-else-if="filtered.length === 0" message="Aucun wallet trouve" />

    <div v-else class="wallets-table">
      <div class="table-header">
        <div class="col-rank">#</div>
        <div class="col-player">Joueur</div>
        <div class="col-balance">Solde</div>
        <div class="col-earned">Gagnes</div>
        <div class="col-spent">Depenses</div>
        <div class="col-updated">Derniere activite</div>
        <div class="col-actions"></div>
      </div>

      <div
        v-for="(wallet, idx) in filtered"
        :key="wallet.user_id"
        class="table-row"
        :class="{ expanded: expandedRow === wallet.user_id }"
      >
        <div class="row-main" @click="toggleRow(wallet.user_id)">
          <div class="col-rank">
            <span class="rank-badge" :class="`rank-${idx < 3 ? idx + 1 : 'default'}`">
              {{ idx + 1 }}
            </span>
          </div>
          <div class="col-player">
            <div class="player-name">{{ wallet.username }}</div>
            <div class="player-id">{{ wallet.user_id }}</div>
          </div>
          <div class="col-balance">
            <span class="coins-big">{{ wallet.coins.toLocaleString() }}</span>
            <span class="coins-unit">coins</span>
          </div>
          <div class="col-earned positive">
            +{{ wallet.total_earned.toLocaleString() }}
          </div>
          <div class="col-spent negative">
            -{{ wallet.total_spent.toLocaleString() }}
          </div>
          <div class="col-updated">
            {{ fmt(wallet.updated_at) }}
          </div>
          <div class="col-actions">
            <span class="chevron">{{ expandedRow === wallet.user_id ? '▼' : '▶' }}</span>
          </div>
        </div>

        <!-- Panel d'actions (expand) -->
        <div v-if="expandedRow === wallet.user_id" class="row-actions">
          <div class="action-panel">
            <h4>Ajuster le solde</h4>
            <div class="form-row">
              <input
                type="number"
                :value="getAmount(wallet.user_id)"
                @input="setAmount(wallet.user_id, Number(($event.target as HTMLInputElement).value))"
                min="1"
                class="input"
                placeholder="Montant"
              />
              <AppButton
                variant="success"
                size="sm"
                :disabled="busy === wallet.user_id"
                @click="credit(wallet)"
              >
                + Crediter
              </AppButton>
              <AppButton
                variant="warning"
                size="sm"
                :disabled="busy === wallet.user_id"
                @click="debit(wallet)"
              >
                − Debiter
              </AppButton>
            </div>
          </div>

          <div class="action-panel">
            <h4>Reset individuel</h4>
            <div class="form-row">
              <input
                type="number"
                :value="getResetBalance(wallet.user_id)"
                @input="setResetBalance(wallet.user_id, Number(($event.target as HTMLInputElement).value))"
                min="0"
                class="input"
                placeholder="Nouveau solde"
              />
              <AppButton
                variant="danger"
                size="sm"
                :disabled="busy === wallet.user_id"
                @click="resetOne(wallet)"
              >
                🔄 Reset ce wallet
              </AppButton>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
/* ═════════════════════════════════════════════════
   Page wrapper
   ═════════════════════════════════════════════════ */
.wallet-page {
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

.kpi-total .kpi-icon {
  background: color-mix(in srgb, #f1c40f 25%, transparent);
}
.kpi-players .kpi-icon {
  background: color-mix(in srgb, #3498db 25%, transparent);
}
.kpi-avg .kpi-icon {
  background: color-mix(in srgb, #9b59b6 25%, transparent);
}
.kpi-top .kpi-icon {
  background: color-mix(in srgb, #e67e22 25%, transparent);
}

/* ═════════════════════════════════════════════════
   Danger zone
   ═════════════════════════════════════════════════ */
.danger-zone {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 18px 20px;
  background: color-mix(in srgb, var(--danger) 6%, var(--surface));
  border: 1px solid color-mix(in srgb, var(--danger) 40%, var(--border));
  border-radius: 12px;
}

.danger-icon {
  font-size: 2rem;
}

.danger-info {
  flex: 1;
}

.danger-info h3 {
  margin: 0 0 4px;
  color: var(--danger);
  font-size: 1.05rem;
  font-weight: 600;
}

.danger-info p {
  margin: 0;
  font-size: 0.85rem;
  color: var(--text-secondary);
}

.danger-actions {
  display: flex;
  gap: 12px;
  align-items: flex-end;
}

.input-group {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.input-group label {
  font-size: 0.7rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

/* ═════════════════════════════════════════════════
   Search bar
   ═════════════════════════════════════════════════ */
.search-bar {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 0 4px;
}

.search-input {
  position: relative;
  flex: 1;
  max-width: 500px;
}

.search-icon {
  position: absolute;
  left: 14px;
  top: 50%;
  transform: translateY(-50%);
  font-size: 0.9rem;
  opacity: 0.6;
}

.search-input .input {
  padding-left: 40px;
  width: 100%;
}

.count {
  color: var(--text-secondary);
  font-size: 0.85rem;
  font-weight: 500;
}

/* ═════════════════════════════════════════════════
   Inputs
   ═════════════════════════════════════════════════ */
.input {
  background: var(--bg);
  color: var(--text);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 14px;
  font-size: 0.9rem;
  font-family: inherit;
  outline: none;
  transition: border-color var(--transition-fast);
}

.input:focus {
  border-color: var(--accent);
}

.input.input-sm {
  padding: 8px 12px;
  width: 120px;
}

/* ═════════════════════════════════════════════════
   Wallets table
   ═════════════════════════════════════════════════ */
.wallets-table {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}

.table-header,
.row-main {
  display: grid;
  grid-template-columns: 60px 2fr 1.2fr 1.2fr 1.2fr 1.4fr 40px;
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

/* Rang */
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
  color: var(--text-secondary);
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

/* Player */
.player-name {
  font-weight: 600;
  font-size: 0.95rem;
  color: var(--text);
}

.player-id {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.72rem;
  color: var(--text-secondary);
  margin-top: 2px;
}

/* Balance */
.col-balance {
  display: flex;
  align-items: baseline;
  gap: 6px;
}

.coins-big {
  font-size: 1.15rem;
  font-weight: 700;
  color: var(--accent);
}

.coins-unit {
  font-size: 0.7rem;
  color: var(--text-secondary);
  text-transform: uppercase;
}

.positive {
  color: var(--success);
  font-weight: 600;
}

.negative {
  color: var(--danger);
  font-weight: 600;
}

.col-updated {
  font-size: 0.82rem;
  color: var(--text-secondary);
}

.col-actions {
  text-align: center;
}

.chevron {
  font-size: 0.75rem;
  color: var(--text-secondary);
  transition: transform var(--transition-fast);
}

.table-row.expanded .chevron {
  color: var(--accent);
}

/* ═════════════════════════════════════════════════
   Expanded actions
   ═════════════════════════════════════════════════ */
.row-actions {
  padding: 16px 20px 20px;
  background: color-mix(in srgb, var(--accent) 3%, var(--bg));
  border-top: 1px dashed var(--border);
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

.action-panel {
  background: var(--surface);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 16px;
}

.action-panel h4 {
  margin: 0 0 12px;
  font-size: 0.78rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  font-weight: 600;
}

.form-row {
  display: flex;
  gap: 10px;
  align-items: center;
}

.form-row .input {
  flex: 1;
  min-width: 0;
}

/* Responsive */
@media (max-width: 1100px) {
  .table-header,
  .row-main {
    grid-template-columns: 40px 2fr 1fr 1fr 40px;
  }
  .col-earned,
  .col-spent {
    display: none;
  }
  .row-actions {
    grid-template-columns: 1fr;
  }
}

@media (max-width: 700px) {
  .table-header,
  .row-main {
    grid-template-columns: 36px 2fr 1fr 30px;
  }
  .col-updated {
    display: none;
  }
}

@media (max-width: 480px) {
  .kpi-grid {
    gap: 10px;
  }
  .kpi-card {
    padding: 12px 14px;
    gap: 12px;
  }
  .kpi-icon {
    font-size: 1.5rem;
    width: 38px;
    height: 38px;
  }
  .hero {
    padding: 14px 16px;
  }
  .hero-title {
    font-size: 1.3rem;
  }
  .hero-subtitle {
    font-size: 0.85rem;
  }
}
</style>
