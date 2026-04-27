<script setup lang="ts">
import { computed, ref } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useNotes } from "@/composables/useNotes";

const { guildIdFilter } = useGuildSelector();
const { notes, lookupUserId, loading, fetch, add, remove } = useNotes();

const draft = ref({
  content: "",
  category: "general",
  authorId: "desktop",
  authorName: "Desktop App",
});

const CATEGORIES = [
  { key: "general", label: "Général", color: "#7F8C8D" },
  { key: "warning", label: "Avertissement", color: "#E67E22" },
  { key: "positive", label: "Positif", color: "#2ECC71" },
  { key: "context", label: "Contexte", color: "#3498DB" },
];

function categoryColor(key: string): string {
  return CATEGORIES.find((c) => c.key === key)?.color ?? "#7F8C8D";
}
function categoryLabel(key: string): string {
  return CATEGORIES.find((c) => c.key === key)?.label ?? key;
}

const canSubmit = computed(
  () =>
    !!guildIdFilter.value &&
    !!lookupUserId.value.trim() &&
    !!draft.value.content.trim(),
);

async function onSearch() {
  await fetch(lookupUserId.value);
}

async function onAdd() {
  if (!canSubmit.value || !guildIdFilter.value) return;
  await add({
    guild_id: guildIdFilter.value,
    user_id: lookupUserId.value.trim(),
    author_id: draft.value.authorId,
    author_name: draft.value.authorName,
    content: draft.value.content.trim(),
    category: draft.value.category,
  });
  draft.value.content = "";
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}
</script>

<template>
  <div class="notes-page">
    <header class="page-header">
      <h1>📝 Notes modération</h1>
      <p class="lede">
        Notes internes attachées à un utilisateur — invisibles pour lui,
        visibles uniquement par les modérateurs. Utile pour le contexte
        long-terme (« déjà signalé pour spam le 12/03 »).
      </p>
    </header>

    <section class="card">
      <h2>Recherche</h2>
      <div class="lookup">
        <input
          v-model="lookupUserId"
          placeholder="ID de l'utilisateur"
          @keyup.enter="onSearch"
        />
        <button class="btn-secondary" @click="onSearch">Rechercher</button>
      </div>
    </section>

    <section v-if="lookupUserId" class="card">
      <h2>Ajouter une note</h2>
      <form class="add-form" @submit.prevent="onAdd">
        <label>
          Catégorie
          <select v-model="draft.category">
            <option v-for="c in CATEGORIES" :key="c.key" :value="c.key">
              {{ c.label }}
            </option>
          </select>
        </label>
        <label class="full">
          Contenu
          <textarea v-model="draft.content" rows="3" required></textarea>
        </label>
        <div class="actions">
          <button type="submit" class="btn-primary" :disabled="!canSubmit">
            Ajouter
          </button>
        </div>
      </form>
    </section>

    <section v-if="lookupUserId" class="card">
      <h2>Notes existantes</h2>
      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="notes.length === 0" class="empty">
        Aucune note pour cet utilisateur.
      </div>
      <ul v-else class="notes-list">
        <li v-for="n in notes" :key="n.id" class="note">
          <div class="note-header">
            <span
              class="category-badge"
              :style="{ backgroundColor: categoryColor(n.category) }"
            >
              {{ categoryLabel(n.category) }}
            </span>
            <span class="note-author">{{ n.author_name }}</span>
            <span class="note-date">{{ formatDate(n.created_at) }}</span>
            <button class="btn-icon" @click="remove(n.id)" title="Supprimer">
              🗑️
            </button>
          </div>
          <div class="note-content">{{ n.content }}</div>
        </li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
.notes-page {
  max-width: 900px;
  margin: 0 auto;
  padding: 24px;
}
.page-header {
  margin-bottom: 24px;
}
.page-header h1 {
  margin: 0 0 8px 0;
  font-size: 1.6rem;
}
.lede {
  color: var(--text-muted, #888);
  margin: 0;
}
.card {
  background: var(--bg-card, #1f1f1f);
  border: 1px solid var(--border-color, #333);
  border-radius: 8px;
  padding: 20px;
  margin-bottom: 20px;
}
.card h2 {
  margin: 0 0 12px 0;
}
.lookup {
  display: flex;
  gap: 8px;
}
.lookup input,
.add-form input,
.add-form select,
.add-form textarea {
  background: var(--bg-input, #2a2a2a);
  border: 1px solid var(--border-color, #444);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
}
.lookup input {
  flex: 1;
  max-width: 320px;
}
.add-form {
  display: flex;
  flex-direction: column;
  gap: 12px;
}
.add-form label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.9rem;
}
.add-form label.full textarea {
  width: 100%;
  resize: vertical;
}
.btn-secondary,
.btn-primary {
  border: none;
  border-radius: 4px;
  padding: 8px 18px;
  cursor: pointer;
  font-weight: 600;
}
.btn-secondary {
  background: var(--bg-input, #2a2a2a);
  color: inherit;
}
.btn-primary {
  background: #5865F2;
  color: white;
}
.btn-primary:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.actions {
  display: flex;
  justify-content: flex-end;
}
.notes-list {
  list-style: none;
  padding: 0;
  margin: 0;
}
.note {
  background: var(--bg-input, #181818);
  border-left: 4px solid var(--border-color, #333);
  padding: 12px 16px;
  margin-bottom: 8px;
  border-radius: 4px;
}
.note-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}
.category-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 12px;
  color: white;
  font-size: 0.75rem;
  font-weight: 600;
}
.note-author {
  font-weight: 600;
  font-size: 0.9rem;
}
.note-date {
  font-size: 0.8rem;
  color: var(--text-muted, #888);
  margin-left: auto;
}
.note-content {
  white-space: pre-wrap;
  word-break: break-word;
}
.btn-icon {
  background: none;
  border: none;
  cursor: pointer;
  font-size: 1rem;
}
.loading,
.empty {
  padding: 16px;
  text-align: center;
  color: var(--text-muted, #888);
}
</style>
