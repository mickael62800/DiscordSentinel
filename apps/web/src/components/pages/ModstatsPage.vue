<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { modstatsService } from "@/services/moderationAdvancedService";
import type { ModStatsEntry } from "@/types/moderation-advanced";

const { guildIdFilter } = useGuildSelector();
const { error: showError } = useToast();
const stats = ref<ModStatsEntry[]>([]);
const loading = ref(true);

async function fetchStats() {
  if (!guildIdFilter.value) {
    stats.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    stats.value = await modstatsService.list(guildIdFilter.value);
  } catch (e) {
    console.error(e);
    showError("Erreur chargement modstats.");
  } finally {
    loading.value = false;
  }
}

onMounted(fetchStats);
watch(guildIdFilter, fetchStats);

const totalActions = computed(() =>
  stats.value.reduce((acc, s) => acc + s.total, 0),
);
const activeMods = computed(() => stats.value.length);
const topMod = computed(() => stats.value[0]);

function medal(idx: number): string {
  return ["🥇", "🥈", "🥉"][idx] ?? `#${idx + 1}`;
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>📊 Modstats — 30 derniers jours</h1>
      <p class="lede">
        Métriques d'activité par modérateur sur les 30 derniers jours :
        warns / mutes / bans / kicks. Top 20 trié par nombre total d'actions.
      </p>
    </header>

    <section class="kpi-row">
      <div class="kpi-card">
        <span class="kpi-value">{{ totalActions }}</span>
        <span class="kpi-label">Total actions</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ activeMods }}</span>
        <span class="kpi-label">Modérateurs actifs</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ topMod?.moderator_name ?? "—" }}</span>
        <span class="kpi-label">Top modérateur</span>
      </div>
    </section>

    <section class="card">
      <h2>Classement</h2>
      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="stats.length === 0" class="empty">
        Aucune action de modération sur les 30 derniers jours.
      </div>
      <table v-else class="table">
        <thead>
          <tr>
            <th>#</th>
            <th>Modérateur</th>
            <th>Total</th>
            <th>Warns</th>
            <th>Mutes</th>
            <th>Bans</th>
            <th>Kicks</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(s, idx) in stats" :key="s.moderator_id">
            <td class="rank">{{ medal(idx) }}</td>
            <td>
              <strong>{{ s.moderator_name }}</strong>
              <small class="muted">{{ s.moderator_id }}</small>
            </td>
            <td><strong>{{ s.total }}</strong></td>
            <td>{{ s.warns }}</td>
            <td>{{ s.mutes }}</td>
            <td>{{ s.bans }}</td>
            <td>{{ s.kicks }}</td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.kpi-row {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin-bottom: 20px;
}
.kpi-card {
  background: var(--bg-card, #1f1f1f);
  border: 1px solid var(--border-color, #333);
  border-radius: 8px;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
}
.kpi-value {
  font-size: 1.6rem;
  font-weight: 700;
}
.kpi-label {
  font-size: 0.85rem;
  color: var(--text-muted, #888);
  margin-top: 4px;
}
.rank {
  font-size: 1.3rem;
  text-align: center;
  width: 60px;
}
</style>
