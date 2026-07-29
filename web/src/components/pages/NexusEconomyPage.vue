<script setup lang="ts">
// Economie Nexus : classement des portefeuilles + detail d'un joueur.
//
// Lecture seule. Les transferts existent cote API mais restent volontairement
// hors interface : creer ou deplacer des coins depuis le web sans double
// validation serait une porte ouverte a l'inflation accidentelle. Ca se fait
// depuis Discord avec /donner, ou ca meritera un ecran dedie et audite.

import { ref, watch } from "vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import {
  nexusEconomyService,
  type NexusTransaction,
  type NexusWallet,
} from "@/services/nexusEconomyService";
import AdminPageShell from "../layouts/AdminPageShell.vue";

const { selectedGuildId, selectedGuild } = useGuildSelector();

const wallets = ref<NexusWallet[]>([]);
const loading = ref(false);
const errorMessage = ref("");

/// Joueur dont l'historique est deplie (un seul a la fois).
const openUserId = ref<string | null>(null);
const history = ref<NexusTransaction[]>([]);
const historyLoading = ref(false);

async function load() {
  if (!selectedGuildId.value) {
    wallets.value = [];
    return;
  }
  loading.value = true;
  errorMessage.value = "";
  openUserId.value = null;
  try {
    wallets.value = await nexusEconomyService.leaderboard(selectedGuildId.value, 50);
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Chargement impossible";
    wallets.value = [];
  } finally {
    loading.value = false;
  }
}

async function toggleHistory(w: NexusWallet) {
  if (openUserId.value === w.user_id) {
    openUserId.value = null;
    return;
  }
  openUserId.value = w.user_id;
  history.value = [];
  historyLoading.value = true;
  try {
    history.value = await nexusEconomyService.history(w.guild_id, w.user_id, 50);
  } catch {
    history.value = [];
  } finally {
    historyLoading.value = false;
  }
}

/// Separateur de milliers : les soldes montent vite et deviennent illisibles.
function fmt(n: number): string {
  return n.toLocaleString("fr-FR");
}

function fmtDate(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("fr-FR");
}

watch(selectedGuildId, load, { immediate: true });
</script>

<template>
  <AdminPageShell
    title="Economie"
    :subtitle="selectedGuild?.name ?? 'Aucun serveur selectionne'"
  >
    <p v-if="!selectedGuildId" class="ne-hint">
      Selectionne un serveur Discord pour voir son economie.
    </p>

    <p v-else-if="errorMessage" class="ne-error">{{ errorMessage }}</p>

    <p v-else-if="loading" class="ne-hint">Chargement…</p>

    <p v-else-if="!wallets.length" class="ne-hint">
      Aucun portefeuille pour l'instant.
    </p>

    <table v-else class="ne-table">
      <thead>
        <tr>
          <th>#</th>
          <th>Joueur</th>
          <th>Solde</th>
          <th>Gagne</th>
          <th>Depense</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        <template v-for="(w, i) in wallets" :key="w.user_id">
          <tr>
            <td class="ne-rank">{{ i + 1 }}</td>
            <td>{{ w.username || w.user_id }}</td>
            <td class="ne-coins">{{ fmt(w.coins) }}</td>
            <td>{{ fmt(w.total_earned) }}</td>
            <td>{{ fmt(w.total_spent) }}</td>
            <td>
              <button type="button" class="ne-link" @click="toggleHistory(w)">
                {{ openUserId === w.user_id ? "Masquer" : "Historique" }}
              </button>
            </td>
          </tr>
          <tr v-if="openUserId === w.user_id" class="ne-history-row">
            <td colspan="6">
              <p v-if="historyLoading" class="ne-hint">Chargement…</p>
              <p v-else-if="!history.length" class="ne-hint">Aucune transaction.</p>
              <ul v-else class="ne-history">
                <li v-for="t in history" :key="t.id">
                  <span class="ne-date">{{ fmtDate(t.created_at) }}</span>
                  <span class="ne-amount" :class="t.amount >= 0 ? 'pos' : 'neg'">
                    {{ t.amount >= 0 ? "+" : "" }}{{ fmt(t.amount) }}
                  </span>
                  <span class="ne-desc">{{ t.description || t.source }}</span>
                  <span class="ne-balance">solde {{ fmt(t.balance_after) }}</span>
                </li>
              </ul>
            </td>
          </tr>
        </template>
      </tbody>
    </table>
  </AdminPageShell>
</template>

<style scoped>
.ne-hint {
  color: var(--text-secondary);
}

.ne-error {
  color: var(--danger);
}

.ne-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.92rem;
}

.ne-table th,
.ne-table td {
  text-align: left;
  padding: var(--space-sm);
  border-bottom: 1px solid var(--bg-hover);
}

.ne-table th {
  color: var(--text-secondary);
  font-weight: 600;
}

.ne-rank {
  color: var(--text-secondary);
  width: 3rem;
}

.ne-coins {
  color: var(--accent);
  font-weight: 600;
}

.ne-link {
  background: none;
  border: none;
  color: var(--text-secondary);
  cursor: pointer;
  text-decoration: underline;
  padding: 0;
}

.ne-link:hover {
  color: var(--text-primary);
}

.ne-history-row td {
  background: var(--bg-secondary);
}

.ne-history {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.ne-history li {
  display: grid;
  grid-template-columns: 12rem 6rem 1fr 9rem;
  gap: var(--space-sm);
  font-size: 0.86rem;
  color: var(--text-secondary);
}

.ne-amount.pos {
  color: var(--success);
}

.ne-amount.neg {
  color: var(--danger);
}

.ne-desc {
  color: var(--text-primary);
}

@media (max-width: 800px) {
  .ne-history li {
    grid-template-columns: 1fr;
    gap: 0;
    padding-bottom: var(--space-xs);
    border-bottom: 1px solid var(--bg-hover);
  }
}
</style>
