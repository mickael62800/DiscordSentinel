<script setup lang="ts">
import { ref } from "vue";
import { walletService } from "@/services/walletService";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";
import type { Wallet } from "../../types";
import AppButton from "../atoms/AppButton.vue";

defineProps<{
  wallets: Wallet[];
  guildId: string;
}>();

const emit = defineEmits<{ changed: [] }>();

const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();
const { success, error: toastError } = useToast();

const expandedRow = ref<string | null>(null);
const busy = ref<string | null>(null);
const amounts = ref<Record<string, number>>({});
const resetBalances = ref<Record<string, number>>({});

function getAmount(userId: string): number { return amounts.value[userId] ?? 100; }
function setAmount(userId: string, val: number) { amounts.value[userId] = val; }
function getResetBalance(userId: string): number { return resetBalances.value[userId] ?? 100; }
function setResetBalance(userId: string, val: number) { resetBalances.value[userId] = val; }

function toggleRow(userId: string) {
  expandedRow.value = expandedRow.value === userId ? null : userId;
}

async function credit(wallet: Wallet, guildId: string) {
  const amount = getAmount(wallet.user_id);
  if (amount <= 0) {
    toastError("Le montant doit etre positif.");
    return;
  }
  busy.value = wallet.user_id;
  try {
    await walletService.credit(guildId, wallet.user_id, amount, "Credit admin desktop");
    success(`+${amount} coins credites a ${wallet.username}`);
    emit("changed");
  } catch (e) {
    toastError(String(e));
  } finally {
    busy.value = null;
  }
}

async function debit(wallet: Wallet, guildId: string) {
  const amount = getAmount(wallet.user_id);
  if (amount <= 0) {
    toastError("Le montant doit etre positif.");
    return;
  }
  busy.value = wallet.user_id;
  try {
    await walletService.debit(guildId, wallet.user_id, amount, "Debit admin desktop");
    success(`-${amount} coins debites de ${wallet.username}`);
    emit("changed");
  } catch (e) {
    toastError(String(e));
  } finally {
    busy.value = null;
  }
}

async function resetOne(wallet: Wallet, guildId: string) {
  const newBalance = getResetBalance(wallet.user_id);
  const ok = await confirm({
    title: "Reset du wallet",
    message: `Reset le wallet de ${wallet.username} a ${newBalance} coins ? Son historique sera efface.`,
  });
  if (!ok) return;

  busy.value = wallet.user_id;
  try {
    await walletService.reset(guildId, wallet.user_id, newBalance);
    success(`Wallet de ${wallet.username} reset a ${newBalance} coins`);
    emit("changed");
  } catch (e) {
    toastError(String(e));
  } finally {
    busy.value = null;
  }
}
</script>

<template>
  <div class="wallets-table">
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
      v-for="(wallet, idx) in wallets"
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
        <div class="col-updated">{{ fmt(wallet.updated_at) }}</div>
        <div class="col-actions">
          <span class="chevron">{{ expandedRow === wallet.user_id ? '▼' : '▶' }}</span>
        </div>
      </div>

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
              variant="success" size="sm"
              :disabled="busy === wallet.user_id"
              @click="credit(wallet, guildId)"
            >+ Crediter</AppButton>
            <AppButton
              variant="warning" size="sm"
              :disabled="busy === wallet.user_id"
              @click="debit(wallet, guildId)"
            >− Debiter</AppButton>
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
              variant="danger" size="sm"
              :disabled="busy === wallet.user_id"
              @click="resetOne(wallet, guildId)"
            >🔄 Reset ce wallet</AppButton>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
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

.table-row { border-bottom: 1px solid var(--border); }
.table-row:last-child { border-bottom: none; }

.row-main { cursor: pointer; transition: background-color var(--transition-fast); }
.row-main:hover { background: color-mix(in srgb, var(--accent) 4%, transparent); }
.table-row.expanded .row-main { background: color-mix(in srgb, var(--accent) 8%, transparent); }

/* Rang */
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

.player-name { font-weight: 600; font-size: 0.95rem; color: var(--text); }
.player-id {
  font-family: "JetBrains Mono", monospace;
  font-size: 0.72rem;
  color: var(--text-secondary);
  margin-top: 2px;
}

.col-balance { display: flex; align-items: baseline; gap: 6px; }
.coins-big { font-size: 1.15rem; font-weight: 700; color: var(--accent); }
.coins-unit { font-size: 0.7rem; color: var(--text-secondary); text-transform: uppercase; }

.positive { color: var(--success); font-weight: 600; }
.negative { color: var(--danger); font-weight: 600; }

.col-updated { font-size: 0.82rem; color: var(--text-secondary); }
.col-actions { text-align: center; }
.chevron {
  font-size: 0.75rem;
  color: var(--text-secondary);
  transition: transform var(--transition-fast);
}
.table-row.expanded .chevron { color: var(--accent); }

/* Expanded */
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

.form-row { display: flex; gap: 10px; align-items: center; }
.form-row .input { flex: 1; min-width: 0; }

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

@media (max-width: 1100px) {
  .table-header, .row-main {
    grid-template-columns: 40px 2fr 1fr 1fr 40px;
  }
  .col-earned, .col-spent { display: none; }
  .row-actions { grid-template-columns: 1fr; }
}

@media (max-width: 700px) {
  .table-header, .row-main {
    grid-template-columns: 36px 2fr 1fr 30px;
  }
  .col-updated { display: none; }
}
</style>
