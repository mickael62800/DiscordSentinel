<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useAuth } from "@/composables/useAuth";
import { useToast } from "@/composables/useToast";
import { useAutomod } from "@/composables/useAutomod";
import { automodService, type AutomodReview, type ResolveActionChoice } from "@/services/automodService";
import { on as onWsEvent } from "@/api/events";

const { guildIdFilter } = useGuildSelector();
const { user } = useAuth();
const { success, error: showError } = useToast();
const { fetchDetections } = useAutomod();

const reviews = ref<AutomodReview[]>([]);
const reviewsLoading = ref(false);
const resolving = ref<string | null>(null);

async function fetchReviews() {
  if (!guildIdFilter.value) {
    reviews.value = [];
    return;
  }
  reviewsLoading.value = true;
  try {
    reviews.value = await automodService.listReviews(guildIdFilter.value);
  } catch (e) {
    console.error(e);
    showError("Echec chargement reviews automod.");
  } finally {
    reviewsLoading.value = false;
  }
}

async function resolveReview(r: AutomodReview, choice: ResolveActionChoice) {
  if (!user.value) {
    showError("Authentification requise.");
    return;
  }
  resolving.value = r.id;
  try {
    await automodService.resolveReview(r.id, {
      applied_action: choice,
      resolved_by_id: user.value.id,
      resolved_by_name: user.value.global_name ?? user.value.username,
    });
    success(`Action "${choice}" appliquee. La carte Discord est mise a jour.`);
    await Promise.all([fetchReviews(), fetchDetections()]);
  } catch (e) {
    showError(`Echec resolution : ${e}`);
  } finally {
    resolving.value = null;
  }
}

onMounted(fetchReviews);
watch(guildIdFilter, fetchReviews);

const offCreated = onWsEvent("ws:automod_review_created", () => fetchReviews());
const offResolved = onWsEvent("ws:automod_review_resolved", () => fetchReviews());
onUnmounted(() => {
  offCreated();
  offResolved();
});
</script>

<template>
  <section class="card reviews-section">
    <div class="reviews-header">
      <h2>🛎️ Reviews en attente <span v-if="reviews.length" class="badge">{{ reviews.length }}</span></h2>
      <button class="btn-secondary" @click="fetchReviews" :disabled="reviewsLoading">↻</button>
    </div>
    <p class="lede small">
      Cartes postées par le bot dans le salon de logs avec boutons (Apply / Warn / Mute / Ban / Ignorer).
      Cliquer ici applique l'action ET grise la carte Discord en temps réel.
    </p>
    <div v-if="reviewsLoading" class="loading">Chargement…</div>
    <div v-else-if="reviews.length === 0" class="empty">Aucune review pending.</div>
    <ul v-else class="reviews-list">
      <li v-for="r in reviews" :key="r.id" class="review-card">
        <div class="review-head">
          <span class="suggested-badge" :class="`sa-${r.suggested_action}`">
            Suggéré : {{ r.suggested_action }}
          </span>
          <span class="muted">score {{ r.score.toFixed(2) }}</span>
          <span class="muted">·</span>
          <strong>{{ r.user_name }}</strong>
          <small class="muted">{{ r.user_id }}</small>
          <span class="muted spacer">{{ new Date(r.created_at).toLocaleString("fr-FR") }}</span>
        </div>
        <div class="review-body">
          <div class="reason"><strong>Raison IA :</strong> {{ r.reason }}</div>
          <div class="content-preview">
            <strong>Message :</strong>
            <pre>{{ r.content_preview }}</pre>
          </div>
        </div>
        <div class="review-actions">
          <button
            v-for="choice in (['warn','delete','mute','ban','ignore'] as ResolveActionChoice[])"
            :key="choice"
            class="action-btn"
            :class="[`btn-${choice}`, { suggested: choice === r.suggested_action }]"
            :disabled="resolving === r.id"
            @click="resolveReview(r, choice)"
          >
            {{ choice }}
          </button>
        </div>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 20px;
}

.reviews-section { margin-bottom: 20px; }
.reviews-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}
.reviews-header h2 { margin: 0; font-size: 1.1rem; }
.badge {
  background: #E67E22;
  color: white;
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 0.75rem;
  margin-left: 6px;
}

.lede.small {
  color: var(--text-secondary);
  font-size: 0.85rem;
  margin: 0 0 12px 0;
}

.btn-secondary {
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 4px;
  padding: 6px 14px;
  cursor: pointer;
}

.loading { padding: 32px; text-align: center; color: var(--text-secondary); }
.empty { color: var(--text-secondary); font-style: italic; }

.reviews-list { list-style: none; padding: 0; margin: 0; }
.review-card {
  background: var(--bg-card);
  border-left: 4px solid #E67E22;
  padding: 12px 14px;
  border-radius: 6px;
  margin-bottom: 10px;
}
.review-head {
  display: flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  font-size: 0.9rem;
  margin-bottom: 8px;
}
.review-head .spacer { margin-left: auto; }
.muted { color: var(--text-secondary); }

.suggested-badge {
  padding: 2px 8px;
  border-radius: 10px;
  font-size: 0.75rem;
  font-weight: 600;
  color: white;
}
.sa-warn { background: #F1C40F; color: #222; }
.sa-delete { background: #95A5A6; }
.sa-mute { background: #E67E22; }
.sa-ban { background: #E74C3C; }

.review-body { font-size: 0.88rem; margin-bottom: 8px; }
.review-body .reason { margin-bottom: 6px; }
.content-preview pre {
  background: #0d0d0d;
  padding: 8px 10px;
  border-radius: 4px;
  white-space: pre-wrap;
  word-break: break-word;
  max-height: 120px;
  overflow-y: auto;
  margin: 4px 0 0;
  font-size: 0.85rem;
}

.review-actions { display: flex; gap: 6px; flex-wrap: wrap; }
.action-btn {
  border: 1px solid var(--border);
  background: transparent;
  color: inherit;
  border-radius: 4px;
  padding: 6px 12px;
  cursor: pointer;
  font-size: 0.85rem;
  text-transform: capitalize;
  transition: all 0.15s;
}
.action-btn:hover:not(:disabled) { background: rgba(255,255,255,0.05); }
.action-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.action-btn.suggested {
  border-color: #2ECC71;
  color: #2ECC71;
  font-weight: 600;
}
.action-btn.btn-ban { border-color: #E74C3C; color: #E74C3C; }
.action-btn.btn-mute { border-color: #E67E22; color: #E67E22; }
.action-btn.btn-warn { border-color: #F1C40F; color: #F1C40F; }
.action-btn.btn-ignore { border-color: #95A5A6; color: #95A5A6; }
</style>
