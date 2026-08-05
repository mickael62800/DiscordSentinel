<script setup lang="ts">
import AppBadge from "../atoms/AppBadge.vue";
import AppSelect from "@/components/atoms/AppSelect.vue";
import ErrorState from "../atoms/ErrorState.vue";
import PaginationBar from "../molecules/PaginationBar.vue";
import { usePagination } from "../../composables/usePagination";
import { useIdeas } from "../../composables/useIdeas";
import {
  IDEA_CATEGORY_LABELS,
  IDEA_STATUS_LABELS,
  type IdeaStatus,
} from "../../services/ideasService";
import { ideaStatusVariant } from "../../utils/variants";

const emit = defineEmits<{ select: [ideaId: string] }>();

const {
  ideas,
  loading,
  error,
  filterStatus,
  filterCategory,
  filterSearch,
  hasActiveFilters,
  countByStatus,
  fetchIdeas,
  resetFilters,
} = useIdeas();

const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedIdeas } =
  usePagination(ideas);

function categoryLabel(value: string): string {
  return IDEA_CATEGORY_LABELS[value] ?? value;
}

function statusLabel(value: string): string {
  return IDEA_STATUS_LABELS[value as IdeaStatus] ?? value;
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString("fr-FR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
  });
}
</script>

<template>
  <div>
    <div class="ideas-header">
      <h1>Idées</h1>
      <div class="ideas-stats">
        <span class="stat"><strong>{{ countByStatus.nouvelle ?? 0 }}</strong> nouvelle(s)</span>
        <span class="stat"><strong>{{ countByStatus.en_discussion ?? 0 }}</strong> en discussion</span>
        <span class="stat"><strong>{{ ideas.length }}</strong> affichée(s)</span>
      </div>
    </div>

    <div class="card ideas-toolbar">
      <div class="filter-grid">
        <div class="filter-field filter-search">
          <label>Recherche</label>
          <input
            v-model="filterSearch"
            type="text"
            placeholder="Titre ou description..."
            @keyup.enter="fetchIdeas"
          />
        </div>
        <div class="filter-field">
          <label>Statut</label>
          <AppSelect v-model="filterStatus">
            <option value="all">Tous</option>
            <option value="nouvelle">Nouvelle</option>
            <option value="en_discussion">En discussion</option>
            <option value="acceptee">Acceptée</option>
            <option value="refusee">Refusée</option>
            <option value="realisee">Réalisée</option>
          </AppSelect>
        </div>
        <div class="filter-field">
          <label>Catégorie</label>
          <AppSelect v-model="filterCategory">
            <option value="all">Toutes</option>
            <option value="evenement">Événement</option>
            <option value="salon">Salon / catégorie</option>
            <option value="role">Rôle</option>
            <option value="bot">Bot / fonctionnalité</option>
            <option value="reglement">Règlement</option>
            <option value="autre">Autre</option>
          </AppSelect>
        </div>
      </div>
      <div class="toolbar-actions">
        <button class="reset-btn" @click="fetchIdeas">Rechercher</button>
        <button v-if="hasActiveFilters" class="reset-btn" @click="resetFilters">
          Réinitialiser
        </button>
      </div>
    </div>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchIdeas" />
    <div v-else-if="loading" class="loading">Chargement...</div>
    <div v-else-if="!ideas.length" class="card empty">
      Aucune idée pour l'instant. Poste le panneau sur Discord avec
      <code>/idee panneau</code> pour lancer la boîte à idées.
    </div>

    <div v-else class="card idea-list">
      <div
        v-for="idea in paginatedIdeas"
        :key="idea.id"
        class="idea-row"
        @click="emit('select', idea.id)"
      >
        <div class="idea-main">
          <span class="idea-title">{{ idea.title }}</span>
          <span class="idea-meta">
            par {{ idea.author_name }} · {{ categoryLabel(idea.category) }} ·
            {{ formatDate(idea.created_at) }}
          </span>
        </div>
        <AppBadge :label="statusLabel(idea.status)" :variant="ideaStatusVariant(idea.status)" />
      </div>
    </div>

    <PaginationBar
      v-if="!loading && !error && ideas.length"
      v-model:current-page="currentPage"
      v-model:per-page="perPage"
      :total-items="totalItems"
      :total-pages="totalPages"
    />
  </div>
</template>

<style scoped>
.ideas-header {
  display: flex;
  align-items: baseline;
  justify-content: space-between;
  gap: 16px;
  flex-wrap: wrap;
}
.ideas-stats {
  display: flex;
  gap: 14px;
  color: var(--text-secondary);
  font-size: 13px;
}
.ideas-toolbar {
  margin: 12px 0;
  padding: 14px;
}
.filter-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(180px, 1fr));
  gap: 12px;
}
.filter-field label {
  display: block;
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 4px;
}
.filter-field input {
  width: 100%;
}
.toolbar-actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
}
.idea-list {
  padding: 0;
}
.idea-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 12px;
  padding: 12px 14px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
}
.idea-row:last-child {
  border-bottom: none;
}
.idea-row:hover {
  background: var(--bg-hover, rgba(127, 127, 127, 0.08));
}
.idea-main {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}
.idea-title {
  font-weight: 600;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}
.idea-meta {
  font-size: 12px;
  color: var(--text-secondary);
}
.empty,
.loading {
  padding: 20px;
  color: var(--text-secondary);
}
</style>
