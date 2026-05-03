<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { walletService } from "@/services/walletService";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import { useComponentVisibility } from "@/composables/useComponentVisibility";
import type { Wallet } from "../../types";
import AppButton from "../atoms/AppButton.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import WalletsTable from "../organisms/WalletsTable.vue";

const { visible } = useComponentVisibility();
const { selectedGuildId } = useGuildSelector();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

const wallets = ref<Wallet[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);
const search = ref("");
const resetAllBalance = ref<number>(100);

const filtered = computed(() => {
  if (!search.value) return wallets.value;
  const q = search.value.toLowerCase();
  return wallets.value.filter(
    (w) => w.username.toLowerCase().includes(q) || w.user_id.includes(q),
  );
});

const totalCoins = computed(() => wallets.value.reduce((sum, w) => sum + w.coins, 0));
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

watch(selectedGuildId, fetchWallets);
onMounted(fetchWallets);
</script>

<template>
  <div class="wallet-page page--wide">
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

    <LoadingState v-if="loading" />
    <ErrorState v-else-if="error" :message="error" @retry="fetchWallets" />
    <EmptyState v-else-if="filtered.length === 0" message="Aucun wallet trouve" />

    <WalletsTable
      v-else
      :wallets="filtered"
      :guild-id="selectedGuildId ?? ''"
      @changed="fetchWallets"
    />
  </div>
</template>

<style scoped>
.wallet-page {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

/* Hero */
.hero {
  display: flex;
  justify-content: space-between;
  align-items: flex-end;
  padding-bottom: 16px;
  border-bottom: 1px solid var(--border);
}
.hero-title {
  display: flex; align-items: center; gap: 12px;
  margin: 0 0 6px;
  font-size: 2rem;
  font-weight: 700;
}
.hero-icon { font-size: 2rem; }
.hero-subtitle { margin: 0; color: var(--text-secondary); font-size: 0.95rem; }

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
  display: flex; align-items: center; gap: 16px;
  transition: transform var(--transition-fast), border-color var(--transition-fast);
}
.kpi-card:hover { transform: translateY(-2px); border-color: var(--accent); }
.kpi-icon {
  font-size: 2rem; width: 48px; height: 48px;
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

.kpi-total .kpi-icon { background: color-mix(in srgb, #f1c40f 25%, transparent); }
.kpi-players .kpi-icon { background: color-mix(in srgb, #3498db 25%, transparent); }
.kpi-avg .kpi-icon { background: color-mix(in srgb, #9b59b6 25%, transparent); }
.kpi-top .kpi-icon { background: color-mix(in srgb, #e67e22 25%, transparent); }

/* Danger zone */
.danger-zone {
  display: flex;
  align-items: center;
  gap: 20px;
  padding: 18px 20px;
  background: color-mix(in srgb, var(--danger) 6%, var(--surface));
  border: 1px solid color-mix(in srgb, var(--danger) 40%, var(--border));
  border-radius: 12px;
}
.danger-icon { font-size: 2rem; }
.danger-info { flex: 1; }
.danger-info h3 {
  margin: 0 0 4px;
  color: var(--danger);
  font-size: 1.05rem;
  font-weight: 600;
}
.danger-info p { margin: 0; font-size: 0.85rem; color: var(--text-secondary); }
.danger-actions { display: flex; gap: 12px; align-items: flex-end; }

.input-group { display: flex; flex-direction: column; gap: 4px; }
.input-group label {
  font-size: 0.7rem; color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: 0.5px;
}

/* Search */
.search-bar {
  display: flex; align-items: center; gap: 16px;
  padding: 0 4px;
}
.search-input { position: relative; flex: 1; max-width: 500px; }
.search-icon {
  position: absolute;
  left: 14px; top: 50%; transform: translateY(-50%);
  font-size: 0.9rem; opacity: 0.6;
}
.search-input .input { padding-left: 40px; width: 100%; }
.count { color: var(--text-secondary); font-size: 0.85rem; font-weight: 500; }

/* Inputs */
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
.input:focus { border-color: var(--accent); }
.input.input-sm { padding: 8px 12px; width: 120px; }

@media (max-width: 768px) {
  .danger-zone {
    flex-direction: column;
    align-items: stretch;
    gap: 12px;
    padding: 14px 16px;
  }
  .danger-actions {
    flex-direction: column;
    align-items: stretch;
    gap: 8px;
  }
}

@media (max-width: 480px) {
  .kpi-grid { gap: 10px; }
  .kpi-card { padding: 12px 14px; gap: 12px; }
  .kpi-icon { font-size: 1.5rem; width: 38px; height: 38px; }
  .hero { padding: 14px 16px; }
  .hero-title { font-size: 1.3rem; }
  .hero-subtitle { font-size: 0.85rem; }
}
</style>
