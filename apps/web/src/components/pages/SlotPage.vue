<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { slotService } from "@/services/casinoService";
import type { SlotSpin, SlotTopWinner, JackpotPool } from "@/types/casino";

const { guildIdFilter } = useGuildSelector();
const { error: showError } = useToast();

const spins = ref<SlotSpin[]>([]);
const topWinners = ref<SlotTopWinner[]>([]);
const jackpot = ref<JackpotPool | null>(null);
const loading = ref(true);

async function fetchAll() {
  if (!guildIdFilter.value) {
    spins.value = [];
    topWinners.value = [];
    jackpot.value = null;
    loading.value = false;
    return;
  }
  loading.value = true;
  const gid = guildIdFilter.value;
  try {
    const [s, t, j] = await Promise.all([
      slotService.recentSpins(gid, 30).catch(() => []),
      slotService.topWinners(gid, 7, 10).catch(() => []),
      slotService.jackpot(gid).catch(() => null),
    ]);
    spins.value = s;
    topWinners.value = t;
    jackpot.value = j;
  } catch (e) {
    console.error(e);
    showError("Erreur chargement slot.");
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

const totalSpins = computed(() => spins.value.length);
const totalJackpots = computed(() => spins.value.filter((s) => s.is_jackpot).length);
const totalPayout = computed(() => spins.value.reduce((a, b) => a + b.payout, 0));
const totalMise = computed(() => spins.value.reduce((a, b) => a + b.mise, 0));
const rtp = computed(() =>
  totalMise.value > 0 ? ((totalPayout.value / totalMise.value) * 100).toFixed(1) : "—",
);
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>🎰 Slot Machine — analytics</h1>
      <p class="lede">
        30 dernières parties + leaderboard 7 jours. Le RTP affiché est calculé
        sur les 30 dernières parties affichées.
      </p>
    </header>

    <section class="kpi-row">
      <div class="kpi-card jackpot">
        <span class="kpi-value">{{ jackpot?.current_pool.toLocaleString() ?? "—" }}</span>
        <span class="kpi-label">Jackpot pool</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ totalSpins }}</span>
        <span class="kpi-label">Spins (récents)</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ totalJackpots }}</span>
        <span class="kpi-label">Jackpots</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ rtp }}%</span>
        <span class="kpi-label">RTP (récent)</span>
      </div>
    </section>

    <div class="grid">
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
              <th>JP</th>
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
              <td>{{ w.jackpot_count }}</td>
              <td>{{ w.spin_count }}</td>
            </tr>
          </tbody>
        </table>
      </section>

      <section class="card">
        <h2>⏱️ Spins récents</h2>
        <div v-if="loading" class="loading">Chargement…</div>
        <div v-else-if="spins.length === 0" class="empty">Aucun spin récent.</div>
        <table v-else class="table">
          <thead>
            <tr>
              <th>Heure</th>
              <th>Joueur</th>
              <th>Symboles</th>
              <th>Mise</th>
              <th>Gain</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="s in spins" :key="s.id" :class="{ jp: s.is_jackpot }">
              <td>{{ formatDate(s.created_at) }}</td>
              <td>{{ s.username }}</td>
              <td class="symbols">{{ s.symbols.join(' ') }}</td>
              <td>{{ s.mise.toLocaleString() }}</td>
              <td>
                <strong v-if="s.is_jackpot">🎰 {{ s.payout.toLocaleString() }}</strong>
                <span v-else>{{ s.payout.toLocaleString() }}</span>
                <small v-if="s.multiplier !== 0" class="muted">×{{ s.multiplier.toFixed(2) }}</small>
              </td>
            </tr>
          </tbody>
        </table>
      </section>
    </div>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.kpi-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
  margin-bottom: 20px;
}
.kpi-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 16px 20px;
}
.kpi-card.jackpot {
  background: linear-gradient(135deg, #f1c40f 0%, #e67e22 100%);
  color: #000;
}
.kpi-value {
  font-size: 1.6rem;
  font-weight: 700;
  display: block;
}
.kpi-label {
  font-size: 0.85rem;
  opacity: 0.9;
}
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}
.symbols {
  font-size: 1.2rem;
  font-family: emoji, sans-serif;
}
tr.jp {
  background: rgba(241, 196, 15, 0.1);
}

@media (max-width: 640px) {
  .kpi-row {
    grid-template-columns: 1fr 1fr;
    gap: 8px;
  }
  .kpi-card {
    padding: 12px 14px;
  }
  .kpi-value {
    font-size: 1.3rem;
  }
  .grid {
    grid-template-columns: 1fr;
    gap: 12px;
  }
}
</style>
