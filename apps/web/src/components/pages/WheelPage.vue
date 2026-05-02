<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { wheelService } from "@/services/casinoService";
import type { WheelSpinLog, WheelTopWinner } from "@/types/casino";

const { guildIdFilter } = useGuildSelector();
const { error: showError } = useToast();

const spins = ref<WheelSpinLog[]>([]);
const topWinners = ref<WheelTopWinner[]>([]);
const loading = ref(true);

async function fetchAll() {
  if (!guildIdFilter.value) {
    spins.value = [];
    topWinners.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  const gid = guildIdFilter.value;
  try {
    const [s, t] = await Promise.all([
      wheelService.recentSpins(gid, 50).catch(() => []),
      wheelService.topWinners(gid, 7, 10).catch(() => []),
    ]);
    spins.value = s;
    topWinners.value = t;
  } catch (e) {
    console.error(e);
    showError("Erreur chargement wheel.");
  } finally {
    loading.value = false;
  }
}

onMounted(fetchAll);
watch(guildIdFilter, fetchAll);

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "2-digit",
    month: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

// Distribution des cases tombees (sur les spins récents).
const distribution = computed(() => {
  const counts = new Map<string, { label: string; count: number; total_payout: number }>();
  for (const s of spins.value) {
    const cur = counts.get(s.case_key) ?? {
      label: s.case_label,
      count: 0,
      total_payout: 0,
    };
    cur.count += 1;
    cur.total_payout += s.payout;
    counts.set(s.case_key, cur);
  }
  return Array.from(counts.entries())
    .map(([case_key, v]) => ({ case_key, ...v }))
    .sort((a, b) => b.count - a.count);
});

const totalSpins = computed(() => spins.value.length);
const totalPayout = computed(() => spins.value.reduce((a, b) => a + b.payout, 0));
const avgPayout = computed(() =>
  totalSpins.value > 0 ? Math.round(totalPayout.value / totalSpins.value) : 0,
);
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>🪙 Roue du Destin — analytics</h1>
      <p class="lede">
        50 derniers spins (toutes guilds) + leaderboard 7 jours +
        distribution des cases tombées sur les spins récents.
      </p>
    </header>

    <section class="kpi-row">
      <div class="kpi-card">
        <span class="kpi-value">{{ totalSpins }}</span>
        <span class="kpi-label">Spins récents</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ totalPayout.toLocaleString() }}c</span>
        <span class="kpi-label">Total payout (récent)</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ avgPayout.toLocaleString() }}c</span>
        <span class="kpi-label">Moyenne / spin</span>
      </div>
    </section>

    <div class="grid">
      <section class="card">
        <h2>🎲 Distribution des cases</h2>
        <div v-if="loading" class="loading">Chargement…</div>
        <div v-else-if="distribution.length === 0" class="empty">
          Aucun spin récent.
        </div>
        <table v-else class="table">
          <thead>
            <tr>
              <th>Case</th>
              <th>Tombée</th>
              <th>%</th>
              <th>Payout total</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="d in distribution" :key="d.case_key">
              <td>
                <strong>{{ d.label }}</strong>
                <small class="muted">{{ d.case_key }}</small>
              </td>
              <td>{{ d.count }}</td>
              <td>{{ ((d.count / totalSpins) * 100).toFixed(1) }}%</td>
              <td>{{ d.total_payout.toLocaleString() }}c</td>
            </tr>
          </tbody>
        </table>
      </section>

      <section class="card">
        <h2>🏆 Top 10 (7 jours)</h2>
        <div v-if="loading" class="loading">Chargement…</div>
        <div v-else-if="topWinners.length === 0" class="empty">
          Aucun gagnant sur 7 jours.
        </div>
        <table v-else class="table">
          <thead>
            <tr>
              <th>#</th>
              <th>Joueur</th>
              <th>Gain total</th>
              <th>Spins</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(w, idx) in topWinners" :key="w.user_id">
              <td>{{ idx + 1 }}</td>
              <td>
                <strong>{{ w.username }}</strong>
                <small class="muted">{{ w.user_id }}</small>
              </td>
              <td><strong>{{ w.total_payout.toLocaleString() }}c</strong></td>
              <td>{{ w.spin_count }}</td>
            </tr>
          </tbody>
        </table>
      </section>
    </div>

    <section class="card">
      <h2>⏱️ Spins récents (50)</h2>
      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="spins.length === 0" class="empty">Aucun spin récent.</div>
      <table v-else class="table">
        <thead>
          <tr>
            <th>Heure</th>
            <th>Joueur</th>
            <th>Case</th>
            <th>Payout</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="s in spins" :key="s.id">
            <td>{{ formatDate(s.created_at) }}</td>
            <td>{{ s.username }}</td>
            <td>
              <strong>{{ s.case_label }}</strong>
              <small class="muted">{{ s.case_key }}</small>
            </td>
            <td>
              <span :class="{ pos: s.payout > 0, neg: s.payout < 0 }">
                {{ s.payout > 0 ? '+' : '' }}{{ s.payout.toLocaleString() }}c
              </span>
            </td>
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
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px 20px;
}
.kpi-value {
  font-size: 1.6rem;
  font-weight: 700;
  display: block;
}
.kpi-label {
  font-size: 0.85rem;
  color: var(--text-secondary);
}
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
  margin-bottom: 20px;
}
.pos {
  color: #2ECC71;
  font-weight: 600;
}
.neg {
  color: #E74C3C;
  font-weight: 600;
}

@media (max-width: 640px) {
  .kpi-row {
    grid-template-columns: 1fr;
    gap: 8px;
  }
  .grid {
    grid-template-columns: 1fr;
    gap: 12px;
  }
}
</style>
