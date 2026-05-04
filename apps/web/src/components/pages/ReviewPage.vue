<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { reviewService } from "@/services/moderationAdvancedService";
import type { ReviewQueueEntry } from "@/types/moderation-advanced";

const { guildIdFilter } = useGuildSelector();
const { success, error: showError } = useToast();

const pending = ref<ReviewQueueEntry[]>([]);
const loading = ref(true);
const resolveDialog = ref<{ id: string; status: "approved" | "rejected" | "changed"; notes: string } | null>(null);

async function fetchPending() {
  if (!guildIdFilter.value) {
    pending.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    pending.value = await reviewService.listPending(guildIdFilter.value);
  } catch (e) {
    console.error(e);
    showError("Erreur chargement reviews.");
  } finally {
    loading.value = false;
  }
}

function startResolve(id: string, status: "approved" | "rejected" | "changed") {
  resolveDialog.value = { id, status, notes: "" };
}

async function confirmResolve() {
  if (!resolveDialog.value) return;
  try {
    await reviewService.resolve(resolveDialog.value.id, {
      status: resolveDialog.value.status,
      reviewer_id: "desktop",
      reviewer_name: "Desktop App",
      reviewer_notes: resolveDialog.value.notes.trim() || null,
    });
    success("Review résolue.");
    resolveDialog.value = null;
    await fetchPending();
  } catch (e) {
    console.error(e);
    showError("Erreur lors de la résolution.");
  }
}

onMounted(fetchPending);
watch(guildIdFilter, fetchPending);

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR");
}
</script>

<template>
  <div class="page page--constrained">
    <header class="page-header">
      <h1>🔍 Reviews modération</h1>
      <p class="lede">
        File d'attente des actions de modération qui requièrent une seconde
        opinion. Les modérateurs créent une review depuis Discord
        (<code>/review add</code>), les seniors valident ici (Approved /
        Rejected / Changed).
      </p>
    </header>

    <section class="card">
      <h2>Reviews en attente</h2>
      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="pending.length === 0" class="empty">
        Aucune review en attente. 🎉
      </div>
      <ul v-else class="reviews-list">
        <li v-for="r in pending" :key="r.id" class="review">
          <div class="review-header">
            <span class="review-action">{{ r.action_type ?? "—" }}</span>
            <span class="review-target">cible : <strong>{{ r.target_name ?? "—" }}</strong></span>
            <span class="review-date">{{ formatDate(r.added_at) }}</span>
          </div>
          <div class="review-body">
            <p v-if="r.action_reason"><em>Raison action :</em> {{ r.action_reason }}</p>
            <p>
              <em>Demandée par</em> <strong>{{ r.added_by_name }}</strong> :
              {{ r.reason ?? "(pas de motif)" }}
            </p>
          </div>
          <div class="review-actions">
            <button class="btn-success" @click="startResolve(r.id, 'approved')">✅ Approuver</button>
            <button class="btn-warn" @click="startResolve(r.id, 'changed')">🔁 À modifier</button>
            <button class="btn-danger" @click="startResolve(r.id, 'rejected')">❌ Rejeter</button>
          </div>
        </li>
      </ul>
    </section>

    <!-- Dialog de résolution -->
    <div v-if="resolveDialog" class="modal-backdrop" @click.self="resolveDialog = null">
      <div class="modal">
        <h3>Confirmer la résolution</h3>
        <p>Statut : <strong>{{ resolveDialog.status }}</strong></p>
        <label>
          Notes du relecteur (optionnel)
          <textarea v-model="resolveDialog.notes" rows="3"></textarea>
        </label>
        <div class="actions">
          <button class="btn-secondary" @click="resolveDialog = null">Annuler</button>
          <button class="btn-primary" @click="confirmResolve">Confirmer</button>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.reviews-list {
  list-style: none;
  padding: 0;
  margin: 0;
}
.review {
  background: var(--bg-card);
  border-left: 4px solid #F1C40F;
  padding: 12px 16px;
  margin-bottom: 12px;
  border-radius: 4px;
}
.review-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
  flex-wrap: wrap;
}
.review-action {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 4px;
  background: var(--bg-secondary);
  font-size: 0.85rem;
  font-weight: 600;
}
.review-target {
  font-size: 0.9rem;
}
.review-date {
  margin-left: auto;
  font-size: 0.8rem;
  color: var(--text-secondary);
}
.review-body {
  margin-bottom: 12px;
}
.review-body p {
  margin: 4px 0;
}
.review-actions {
  display: flex;
  gap: 8px;
}
.modal-backdrop {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}
.modal {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 24px;
  width: 90%;
  max-width: 500px;
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.modal h3 {
  margin: 0;
}
.modal label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.9rem;
}
.modal textarea {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
  resize: vertical;
}
</style>
