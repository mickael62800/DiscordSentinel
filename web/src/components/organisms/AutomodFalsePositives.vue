<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { automodService, type FpStats } from "@/services/automodService";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";

const { guildIdFilter } = useGuildSelector();

const WINDOWS = [7, 30, 90] as const;
const days = ref<number>(30);
const stats = ref<FpStats | null>(null);
const loading = ref(false);

/** Libelles FR des flags detecteurs connus (fallback = cle brute). */
const FLAG_LABELS: Record<string, string> = {
  spam: "Spam",
  insult: "Insulte",
  link: "Lien",
  phishing: "Phishing",
  nsfw: "NSFW",
};

function flagLabel(flag: string): string {
  return FLAG_LABELS[flag] ?? flag;
}

function pct(rate: number): string {
  return `${Math.round(rate * 1000) / 10}%`;
}

async function fetchStats() {
  const { error: showError } = useToast();
  if (!guildIdFilter.value) {
    stats.value = null;
    return;
  }
  loading.value = true;
  try {
    stats.value = await automodService.fpStats(guildIdFilter.value, days.value);
  } catch (e) {
    console.error("Erreur chargement stats faux positifs :", e);
    showError("Impossible de charger les statistiques de faux positifs.");
    stats.value = null;
  } finally {
    loading.value = false;
  }
}

const overallRate = computed(() => (stats.value ? pct(stats.value.overall.fp_rate) : "—"));

watch([guildIdFilter, days], fetchStats, { immediate: true });
</script>

<template>
  <section class="fp-section">
    <div class="fp-head">
      <h2>Qualité des détections — faux positifs</h2>
      <div class="window-select">
        <button
          v-for="w in WINDOWS"
          :key="w"
          type="button"
          class="win-btn"
          :class="{ active: days === w }"
          @click="days = w"
        >
          {{ w }} j
        </button>
      </div>
    </div>

    <p class="fp-lede">
      Part des détections où l'automod a suggéré une sanction mais où les
      modérateurs ont tranché plus clément (rétrogradé ou ignoré).
    </p>

    <div v-if="loading" class="empty">Chargement…</div>
    <div v-else-if="!stats || stats.overall.total === 0" class="empty">
      Aucune détection résolue sur la fenêtre.
    </div>

    <template v-else>
      <section class="kpi-row">
        <div class="kpi-card">
          <span class="kpi-value">{{ overallRate }}</span>
          <span class="kpi-label">Taux de faux positifs</span>
        </div>
        <div class="kpi-card">
          <span class="kpi-value">{{ stats.overall.overturned }}</span>
          <span class="kpi-label">Détections rétrogradées</span>
        </div>
        <div class="kpi-card">
          <span class="kpi-value">{{ stats.overall.ignored }}</span>
          <span class="kpi-label">Ignorées</span>
        </div>
        <div class="kpi-card">
          <span class="kpi-value">{{ stats.overall.total }}</span>
          <span class="kpi-label">Détections résolues</span>
        </div>
      </section>

      <p v-if="stats.capped" class="capped-note">
        Échantillon tronqué — statistiques approximatives.
      </p>

      <section class="card">
        <h3>Par détecteur (trié par taux de faux positifs)</h3>
        <div v-if="stats.by_flag.length === 0" class="empty">
          Aucun détecteur actif sur la fenêtre.
        </div>
        <ul v-else class="bar-list">
          <li v-for="f in stats.by_flag" :key="f.flag" class="bar-row">
            <div class="bar-label">
              <span class="flag-name">{{ flagLabel(f.flag) }}</span>
              <span class="flag-meta">
                {{ f.overturned }} / {{ f.total }} · {{ pct(f.fp_rate) }}
              </span>
            </div>
            <div class="bar-track">
              <div class="bar-fill" :style="{ width: pct(f.fp_rate) }"></div>
            </div>
          </li>
        </ul>
      </section>
    </template>
  </section>
</template>

<style scoped>
.fp-section {
  margin-bottom: 20px;
}
.fp-head {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.fp-head h2 {
  margin: 0;
  font-size: 1.1rem;
}
.fp-lede {
  color: var(--text-secondary);
  font-size: 0.9rem;
  margin: 6px 0 16px;
}
.window-select {
  display: flex;
  gap: 4px;
}
.win-btn {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 4px 10px;
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 0.85rem;
}
.win-btn.active {
  color: var(--accent);
  border-color: var(--accent);
  font-weight: 600;
}

.kpi-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 12px;
  margin-bottom: 16px;
}
.kpi-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
}
.kpi-value {
  font-size: 1.8rem;
  font-weight: 700;
}
.kpi-label {
  font-size: 0.85rem;
  color: var(--text-secondary);
  margin-top: 4px;
}

.capped-note {
  color: var(--text-secondary);
  font-style: italic;
  font-size: 0.85rem;
  margin: 0 0 12px;
}

.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 20px;
}
.card h3 {
  margin: 0 0 14px 0;
  font-size: 1rem;
}
.empty {
  color: var(--text-secondary);
  font-style: italic;
}

.bar-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.bar-label {
  display: flex;
  justify-content: space-between;
  align-items: baseline;
  margin-bottom: 4px;
}
.flag-name {
  font-weight: 600;
}
.flag-meta {
  font-size: 0.8rem;
  color: var(--text-secondary);
}
.bar-track {
  height: 8px;
  background: var(--bg-primary, rgba(127, 127, 127, 0.15));
  border-radius: var(--radius-sm);
  overflow: hidden;
}
.bar-fill {
  height: 100%;
  background: var(--accent);
  border-radius: var(--radius-sm);
  min-width: 2px;
  transition: width 0.2s ease;
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
    font-size: 1.4rem;
  }
  .card {
    padding: 14px;
  }
}
</style>
