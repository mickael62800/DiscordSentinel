<script setup lang="ts">
import { computed, ref } from "vue";

import AppBadge from "../atoms/AppBadge.vue";
import AppTextarea from "@/components/atoms/AppTextarea.vue";
import ErrorState from "../atoms/ErrorState.vue";
import { useIdeaDetail } from "../../composables/useIdeas";
import { useToast } from "../../composables/useToast";
import {
  IDEA_CATEGORY_LABELS,
  IDEA_STATUS_LABELS,
  type IdeaStatus,
} from "../../services/ideasService";
import { ideaStatusVariant } from "../../utils/variants";

const props = defineProps<{ ideaId: string }>();
const emit = defineEmits<{ back: [] }>();

const { success, error: toastError } = useToast();
const { detail, loading, error, saving, fetchDetail, decide } = useIdeaDetail(
  () => props.ideaId,
);

const reason = ref("");

const idea = computed(() => detail.value?.idea ?? null);

/**
 * Transitions autorisees, miroir de `IdeaStatus::can_transition` cote Rust :
 * une idee réalisée est terminale, et « Réalisée » suppose une acceptation.
 * Le backend reste l'autorite — ici on evite juste de proposer l'impossible.
 */
const availableStatuses = computed<IdeaStatus[]>(() => {
  const current = idea.value?.status;
  if (!current || current === "realisee") return [];
  const all: IdeaStatus[] = ["en_discussion", "acceptee", "refusee"];
  return current === "acceptee" ? [...all, "realisee"] : all;
});

async function applyStatus(status: IdeaStatus) {
  try {
    await decide(status, reason.value);
    reason.value = "";
    success(`Idée marquée « ${IDEA_STATUS_LABELS[status]} ». Le bot prévient l'auteur.`);
  } catch (e) {
    console.error("Erreur decision idee:", e);
    toastError(e instanceof Error ? e.message : "Impossible d'appliquer cette décision");
  }
}

function formatDateTime(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR");
}
</script>

<template>
  <div>
    <button class="back-btn" @click="emit('back')">← Retour aux idées</button>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchDetail" />
    <div v-else-if="loading" class="loading">Chargement...</div>

    <template v-else-if="idea">
      <div class="card idea-head">
        <div class="idea-head-top">
          <h1>{{ idea.title }}</h1>
          <AppBadge
            :label="IDEA_STATUS_LABELS[idea.status] ?? idea.status"
            :variant="ideaStatusVariant(idea.status)"
          />
        </div>
        <p class="idea-meta">
          Proposée par <strong>{{ idea.author_name }}</strong> ·
          {{ IDEA_CATEGORY_LABELS[idea.category] ?? idea.category }} ·
          {{ formatDateTime(idea.created_at) }}
        </p>
        <p class="idea-description">{{ idea.description }}</p>
        <p v-if="idea.channel_id" class="idea-channel">
          Salon de discussion : <code>#{{ idea.channel_id }}</code>
        </p>
        <div v-if="idea.decided_at" class="idea-decision">
          Décision de <strong>{{ idea.decided_by_name ?? "Staff" }}</strong> le
          {{ formatDateTime(idea.decided_at) }}
          <span v-if="idea.decision_reason"> — {{ idea.decision_reason }}</span>
        </div>
      </div>

      <div class="card idea-actions">
        <h2>Décision du staff</h2>
        <p v-if="!availableStatuses.length" class="muted">
          Cette idée est réalisée : son statut est définitif.
        </p>
        <template v-else>
          <label>
            Motif (envoyé à l'auteur, facultatif)
            <AppTextarea v-model="reason" :rows="3" />
          </label>
          <div class="action-buttons">
            <button
              v-for="status in availableStatuses"
              :key="status"
              :disabled="saving"
              class="status-btn"
              @click="applyStatus(status)"
            >
              {{ IDEA_STATUS_LABELS[status] }}
            </button>
          </div>
        </template>
      </div>

      <div class="card idea-thread">
        <h2>Discussion ({{ detail?.messages.length ?? 0 }})</h2>
        <p v-if="!detail?.messages.length" class="muted">
          Aucun message synchronisé depuis le salon Discord pour l'instant.
        </p>
        <div v-for="m in detail?.messages ?? []" :key="m.id" class="message">
          <div class="message-head">
            <strong>{{ m.author_name }}</strong>
            <span class="role">{{ m.author_role }}</span>
            <span class="date">{{ formatDateTime(m.created_at) }}</span>
          </div>
          <p class="message-body">{{ m.content }}</p>
        </div>
      </div>
    </template>
  </div>
</template>

<style scoped>
.back-btn {
  margin-bottom: 12px;
}
.idea-head-top {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  flex-wrap: wrap;
}
.idea-head h1 {
  margin: 0;
  font-size: 20px;
}
.idea-meta,
.idea-channel {
  color: var(--text-secondary);
  font-size: 13px;
}
.idea-description {
  white-space: pre-wrap;
}
.idea-decision {
  margin-top: 8px;
  padding: 8px 10px;
  border-left: 3px solid var(--accent);
  background: var(--bg-secondary);
  border-radius: var(--radius-sm, 6px);
  font-size: 13px;
}
.idea-actions,
.idea-thread {
  margin-top: 12px;
}
.idea-actions h2,
.idea-thread h2 {
  font-size: 15px;
  margin-top: 0;
}
.action-buttons {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
  margin-top: 10px;
}
.message {
  padding: 10px 0;
  border-bottom: 1px solid var(--border);
}
.message:last-child {
  border-bottom: none;
}
.message-head {
  display: flex;
  gap: 8px;
  align-items: baseline;
  font-size: 12px;
  color: var(--text-secondary);
}
.message-body {
  margin: 4px 0 0;
  white-space: pre-wrap;
}
.muted {
  color: var(--text-secondary);
}
.loading {
  padding: 20px;
  color: var(--text-secondary);
}
</style>
