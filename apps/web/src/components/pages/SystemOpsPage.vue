<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { useToast } from "@/composables/useToast";
import { systemOpsService } from "@/services/polishServices";
import type { CacheStats, ModelInfo } from "@/types/polish";

const { success, error: showError } = useToast();

const models = ref<ModelInfo[]>([]);
const cacheStats = ref<CacheStats | null>(null);
const loading = ref(true);

let pollInterval: number | null = null;

async function fetchAll() {
  try {
    const [m, c] = await Promise.all([
      systemOpsService.getModelsStatus(),
      systemOpsService.getCacheStats(),
    ]);
    models.value = m.models;
    cacheStats.value = c;
  } catch (e) {
    console.error(e);
    showError("Erreur chargement system ops.");
  } finally {
    loading.value = false;
  }
}

async function reloadModel(modelType: string) {
  if (!confirm(`Recharger le modèle ${modelType} à chaud ?`)) return;
  try {
    await systemOpsService.reloadModel(modelType);
    success(`Modèle ${modelType} rechargé.`);
    await fetchAll();
  } catch (e) {
    console.error(e);
    showError(`Erreur reload ${modelType}.`);
  }
}

onMounted(() => {
  fetchAll();
  // Auto-refresh toutes les 10s pour suivre l'évolution.
  pollInterval = window.setInterval(fetchAll, 10_000);
});
onUnmounted(() => {
  if (pollInterval !== null) clearInterval(pollInterval);
});
</script>

<template>
  <div class="page page--constrained">
    <header class="page-header">
      <h1>🛠️ System Operations</h1>
      <p class="lede">
        Surveillance des modèles IA chargés et statistiques du cache Redis.
        Refresh auto toutes les 10s.
      </p>
    </header>

    <div v-if="loading" class="loading">Chargement…</div>

    <div v-else class="grid">
      <!-- ── Models IA ── -->
      <section class="card">
        <h2>🧠 Modèles IA</h2>
        <table v-if="models.length > 0" class="table">
          <thead>
            <tr>
              <th>Nom</th>
              <th>Type</th>
              <th>Statut</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="m in models" :key="m.model_type">
              <td>{{ m.name }}</td>
              <td><code>{{ m.model_type }}</code></td>
              <td>
                <span
                  class="badge"
                  :style="{ backgroundColor: m.loaded ? '#2ECC71' : '#E74C3C' }"
                >
                  {{ m.loaded ? 'Chargé' : 'Non chargé' }}
                </span>
              </td>
              <td>
                <button class="btn-secondary" @click="reloadModel(m.model_type)">
                  Reload
                </button>
              </td>
            </tr>
          </tbody>
        </table>
        <div v-else class="empty">Aucun modèle configuré.</div>
      </section>

      <!-- ── Cache stats ── -->
      <section class="card">
        <h2>⚡ Cache Redis</h2>
        <div v-if="!cacheStats" class="empty">
          Aucune statistique de cache disponible.
        </div>
        <div v-else class="cache-stats">
          <div class="stat-row">
            <span>Hit rate</span>
            <strong class="hit-rate" :class="{ low: cacheStats.hit_rate_percent < 50 }">
              {{ cacheStats.hit_rate_percent.toFixed(1) }}%
            </strong>
          </div>
          <div class="stat-row">
            <span>Hits</span>
            <strong>{{ cacheStats.hits.toLocaleString() }}</strong>
          </div>
          <div class="stat-row">
            <span>Misses</span>
            <strong>{{ cacheStats.misses.toLocaleString() }}</strong>
          </div>
          <div class="stat-row">
            <span>Total requêtes</span>
            <strong>{{ cacheStats.total.toLocaleString() }}</strong>
          </div>

          <div class="hit-bar">
            <div
              class="hit-bar-fill"
              :style="{ width: cacheStats.hit_rate_percent + '%' }"
            ></div>
          </div>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
@import "./_admin-page-shared.css";
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}
.cache-stats {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.stat-row {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
  border-bottom: 1px solid var(--border);
}
.stat-row:last-child {
  border-bottom: none;
}
.hit-rate {
  color: #2ECC71;
}
.hit-rate.low {
  color: #E67E22;
}
.hit-bar {
  height: 8px;
  background: var(--bg-card);
  border-radius: 4px;
  overflow: hidden;
  margin-top: 8px;
}
.hit-bar-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), #2ECC71);
  transition: width 0.3s ease;
}

@media (max-width: 768px) {
  table {
    display: block;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    white-space: nowrap;
    font-size: 12px;
    width: 100%;
  }
  table th,
  table td {
    padding: 6px 8px !important;
  }
}
</style>
