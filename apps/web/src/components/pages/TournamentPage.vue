<script setup lang="ts">
import { onMounted, watch } from "vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useTournaments } from "../../composables/useTournaments";
import { useFormatDate } from "../../composables/useFormatDate";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";

const { selectedGuildId } = useGuildSelector();
const { current, history, loading, error, loadAll } = useTournaments();
const { formatShortDateTime: fmt } = useFormatDate();

async function reload() {
  if (selectedGuildId.value) {
    await loadAll(selectedGuildId.value);
  }
}

onMounted(reload);
watch(selectedGuildId, reload);
</script>

<template>
  <section class="tournaments-page">
    <header class="page-header">
      <h1>Tournoi hebdomadaire</h1>
      <p class="subtitle">
        Classement des gains nets cette semaine (lundi 00h → dimanche 23:59 UTC).
        Le joueur en tete decroche un pourcentage de la caisse communautaire.
      </p>
    </header>

    <LoadingState v-if="loading" />
    <ErrorState v-else-if="error" :message="error" @retry="reload" />

    <template v-else>
      <div v-if="current" class="current-card">
        <div class="current-head page--constrained">
          <div>
            <div class="label">Semaine en cours</div>
            <div class="dates">
              {{ fmt(current.week_start) }} → {{ fmt(current.week_end) }}
            </div>
          </div>
          <div class="prize">
            <div class="label">Prize pool estime</div>
            <div class="amount">
              {{ current.prize_pool_estimated.toLocaleString() }} coins
            </div>
          </div>
        </div>

        <EmptyState
          v-if="!current.standings.length"
          title="Pas encore de participants"
          description="Personne n'a encore gagne ou perdu cette semaine."
        />
        <table v-else class="standings">
          <thead>
            <tr>
              <th>Rang</th>
              <th>Joueur</th>
              <th class="num">Gain net</th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="row in current.standings"
              :key="row.user_id"
              :class="{ leader: row.rank === 1 }"
            >
              <td>#{{ row.rank }}</td>
              <td>{{ row.username }}</td>
              <td class="num" :class="{ pos: row.net_gain > 0, neg: row.net_gain < 0 }">
                {{ row.net_gain.toLocaleString() }}
              </td>
            </tr>
          </tbody>
        </table>
      </div>

      <section class="history-section">
        <h2>Tournois resolus</h2>
        <EmptyState
          v-if="!history.length"
          title="Aucun tournoi resolu"
          description="Les resultats apparaitront ici apres le premier dimanche."
        />
        <table v-else class="history">
          <thead>
            <tr>
              <th>Semaine</th>
              <th>Gagnant</th>
              <th class="num">Gain net</th>
              <th class="num">Prize</th>
              <th>Resolu</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="t in history" :key="t.id">
              <td>{{ fmt(t.week_start) }}</td>
              <td>{{ t.winner_username ?? "—" }}</td>
              <td class="num">{{ t.winner_net_gain.toLocaleString() }}</td>
              <td class="num">{{ t.prize_amount.toLocaleString() }}</td>
              <td>{{ t.resolved_at ? fmt(t.resolved_at) : "—" }}</td>
            </tr>
          </tbody>
        </table>
      </section>
    </template>
  </section>
</template>

<style scoped>
.tournaments-page {
  padding: 1.5rem;
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}
.page-header h1 {
  margin: 0 0 0.25rem 0;
}
.subtitle {
  color: var(--text-secondary);
  margin: 0;
}
.current-card {
  background: var(--card-bg, #1b1e28);
  border: 1px solid var(--border, #2a2f3a);
  border-radius: 8px;
  padding: 1.25rem;
}
.current-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 1rem;
  gap: 1rem;
  flex-wrap: wrap;
}
.label {
  font-size: 0.8rem;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.dates {
  font-weight: 600;
}
.prize .amount {
  font-size: 1.5rem;
  font-weight: 700;
  color: var(--accent, #ffd166);
}
table {
  width: 100%;
  border-collapse: collapse;
}
th,
td {
  padding: 0.5rem 0.75rem;
  text-align: left;
  border-bottom: 1px solid var(--border, #2a2f3a);
}
th.num,
td.num {
  text-align: right;
  font-variant-numeric: tabular-nums;
}
tr.leader td {
  background: rgba(255, 209, 102, 0.08);
  font-weight: 600;
}
.pos {
  color: #4ade80;
}
.neg {
  color: #f87171;
}
.history-section h2 {
  margin-bottom: 0.75rem;
}
</style>
