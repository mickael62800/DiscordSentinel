<script setup lang="ts">
// Coussin : supervision des joueurs (classement + statistiques de jeu).
//
// Lecture seule assumee. Les actions (combats, vols, primes, paris) sont des
// interactions entre joueurs et restent sur Discord : les rejouer depuis un
// back-office fausserait le jeu et contournerait ses regles (mises, cooldowns,
// consentement de l'adversaire).
//
// L'objectif ici est de reperer ce qui ne se voit pas dans Discord : un joueur
// qui decroche, un voleur en serie, une accumulation anormale de coins.

import { computed, ref, watch } from "vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { nexusCoussinService, type CoussinProfile } from "@/services/nexusCoussinService";
import AdminPageShell from "../layouts/AdminPageShell.vue";

const { selectedGuildId, selectedGuild } = useGuildSelector();

const players = ref<CoussinProfile[]>([]);
const loading = ref(false);
const errorMessage = ref("");
const search = ref("");

const filtered = computed(() => {
  const q = search.value.trim().toLowerCase();
  if (!q) return players.value;
  return players.value.filter(
    (p) => p.username.toLowerCase().includes(q) || p.user_id.includes(q),
  );
});

async function load() {
  if (!selectedGuildId.value) {
    players.value = [];
    return;
  }
  loading.value = true;
  errorMessage.value = "";
  try {
    players.value = await nexusCoussinService.ranking(selectedGuildId.value, 100);
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Chargement impossible";
    players.value = [];
  } finally {
    loading.value = false;
  }
}

function fmt(n: number): string {
  return n.toLocaleString("fr-FR");
}

/// Ratio victoires/defaites, en evitant la division par zero.
function ratio(p: CoussinProfile): string {
  const total = p.total_wins + p.total_losses;
  if (total === 0) return "—";
  return `${Math.round((p.total_wins / total) * 100)} %`;
}

/// Part de PV restants : sert a reperer d'un coup d'oeil les joueurs au tapis.
function hpPercent(p: CoussinProfile): number {
  if (p.hp_max <= 0) return 0;
  return Math.max(0, Math.min(100, Math.round((p.hp_current / p.hp_max) * 100)));
}

watch(selectedGuildId, load, { immediate: true });
</script>

<template>
  <AdminPageShell
    title="Coussin Piégé"
    :subtitle="selectedGuild?.name ?? 'Aucun serveur selectionne'"
  >
    <p v-if="!selectedGuildId" class="nc-hint">
      Selectionne un serveur Discord pour voir qui squatte le canape.
    </p>

    <p v-else-if="errorMessage" class="nc-error">{{ errorMessage }}</p>

    <p v-else-if="loading" class="nc-hint">Chargement…</p>

    <p v-else-if="!players.length" class="nc-hint">
      Personne sur le canape pour l'instant. Les profils se creent au premier
      <code>/coussin</code> sur Discord.
    </p>

    <template v-else>
      <input
        v-model="search"
        type="search"
        class="nc-search"
        placeholder="Rechercher un joueur…"
      />

      <table class="nc-table">
        <thead>
          <tr>
            <th>#</th>
            <th>Joueur</th>
            <th>Maniere</th>
            <th>Niveau</th>
            <th>Confort</th>
            <th>Assis / Piege / Nul</th>
            <th>Reussite</th>
            <th>Trouve sous les coussins</th>
            <th>Coins</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(p, i) in filtered" :key="p.user_id">
            <td class="nc-rank">{{ i + 1 }}</td>
            <td>
              <span class="nc-name">{{ p.username || p.user_id }}</span>
              <span v-if="p.title" class="nc-title">{{ p.title }}</span>
            </td>
            <td>{{ p.class }}</td>
            <td>{{ p.level }}</td>
            <td>
              <div class="nc-hp" :title="`${p.hp_current} / ${p.hp_max}`">
                <div class="nc-hp-fill" :style="{ width: hpPercent(p) + '%' }" />
              </div>
            </td>
            <td>{{ p.total_wins }} / {{ p.total_losses }} / {{ p.total_draws }}</td>
            <td>{{ ratio(p) }}</td>
            <td>{{ fmt(p.total_stolen) }}</td>
            <td class="nc-coins">{{ fmt(p.coins) }}</td>
          </tr>
        </tbody>
      </table>

      <p v-if="!filtered.length" class="nc-hint">Aucun joueur ne correspond.</p>
    </template>
  </AdminPageShell>
</template>

<style scoped>
.nc-hint {
  color: var(--text-secondary);
}

.nc-error {
  color: var(--danger);
}

.nc-search {
  width: 100%;
  max-width: 22rem;
  margin-bottom: var(--space-md);
  padding: var(--space-xs) var(--space-sm);
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-md);
  color: var(--text-primary);
}

.nc-search:focus {
  outline: none;
  border-color: var(--accent);
}

.nc-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 0.92rem;
}

.nc-table th,
.nc-table td {
  text-align: left;
  padding: var(--space-sm);
  border-bottom: 1px solid var(--bg-hover);
}

.nc-table th {
  color: var(--text-secondary);
  font-weight: 600;
}

.nc-rank {
  color: var(--text-secondary);
  width: 3rem;
}

.nc-name {
  color: var(--text-primary);
  font-weight: 600;
}

.nc-title {
  display: block;
  font-size: 0.78rem;
  color: var(--text-secondary);
}

.nc-hp {
  width: 5rem;
  height: 6px;
  border-radius: var(--radius-xs);
  background: var(--bg-hover);
  overflow: hidden;
}

.nc-hp-fill {
  height: 100%;
  background: var(--success);
}

.nc-coins {
  color: var(--accent);
  font-weight: 600;
}
</style>
