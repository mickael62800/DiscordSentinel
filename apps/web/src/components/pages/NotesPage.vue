<script setup lang="ts">
import { computed, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useNotes } from "@/composables/useNotes";

interface Props {
  /** Quand true, cache le bloc de recherche (utilise quand monte dans le sous-onglet
   *  "Notes & Preuves" ou un seul champ ID user est expose au niveau parent). */
  embedded?: boolean;
}
const props = defineProps<Props>();

const { guildIdFilter } = useGuildSelector();
const { notes, lookupUserId, loading, fetch, add, remove } = useNotes();

// En mode embedded, fetch automatiquement quand l'ID partage change.
watch(lookupUserId, (id) => {
  if (props.embedded && id.trim()) void fetch(id);
});

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
    <header v-if="!props.embedded" class="page-header">
      <h1>📝 Notes modération</h1>
      <p class="lede">
        Notes internes attachées à un utilisateur — invisibles pour lui,
        visibles uniquement par les modérateurs. Utile pour le contexte
        long-terme (« déjà signalé pour spam le 12/03 »).
      </p>
    </header>

    <section v-if="!props.embedded" class="card">
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
}
.page-header {
  margin-bottom: 24px;
}
.page-header h1 {
  margin: 0 0 8px 0;
  font-size: 22px;
}
.lede {
  color: var(--text-secondary);
  margin: 0;
  font-size: 13px;
}

.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg, 12px);
  padding: 20px;
  margin-bottom: 20px;
}
.card h2 {
  margin: 0 0 16px 0;
  font-size: 16px;
  font-weight: 700;
}

/* ── Inputs / select / textarea ────────────── */
.lookup {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}
.lookup input,
.add-form input,
.add-form select,
.add-form textarea {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md, 8px);
  padding: 8px 12px;
  color: var(--text-primary);
  font-family: inherit;
  font-size: 13px;
  font-weight: 500;
  outline: none;
  transition: border-color var(--transition-fast, 0.15s),
    box-shadow var(--transition-fast, 0.15s);
}
.lookup input:hover,
.add-form input:hover,
.add-form select:hover,
.add-form textarea:hover {
  border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
}
.lookup input:focus,
.add-form input:focus,
.add-form select:focus,
.add-form textarea:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent);
}
.lookup input {
  flex: 1;
  min-width: 220px;
  max-width: 320px;
}

.add-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
}
.add-form label {
  display: flex;
  flex-direction: column;
  gap: 6px;
  font-size: 11px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.6px;
  color: var(--text-secondary);
}
.add-form label.full textarea {
  width: 100%;
  resize: vertical;
  min-height: 80px;
  font-weight: 500;
  text-transform: none;
  letter-spacing: 0;
  color: var(--text-primary);
}

/* ── Buttons ──────────────────────────────── */
.btn-secondary,
.btn-primary {
  border: 1px solid transparent;
  border-radius: var(--radius-md, 8px);
  padding: 8px 18px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  transition: background-color var(--transition-fast, 0.15s),
    border-color var(--transition-fast, 0.15s),
    color var(--transition-fast, 0.15s),
    box-shadow var(--transition-fast, 0.15s);
}
.btn-secondary {
  background: var(--bg-card);
  border-color: var(--border);
  color: var(--text-primary);
}
.btn-secondary:hover:not(:disabled) {
  background: var(--bg-hover);
  border-color: color-mix(in srgb, var(--accent) 50%, var(--border));
}
.btn-primary {
  background: var(--accent);
  color: white;
}
.btn-primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--accent) 88%, white);
  box-shadow: 0 4px 14px color-mix(in srgb, var(--accent) 35%, transparent);
}
.btn-primary:disabled,
.btn-secondary:disabled {
  opacity: 0.55;
  cursor: not-allowed;
  box-shadow: none;
}
.btn-icon {
  width: 30px;
  height: 30px;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px);
  color: var(--text-secondary);
  cursor: pointer;
  font-size: 14px;
  transition: all var(--transition-fast, 0.15s);
}
.btn-icon:hover {
  color: var(--danger);
  border-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 10%, transparent);
}

.actions {
  display: flex;
  justify-content: flex-end;
}

/* ── Notes list ───────────────────────────── */
.notes-list {
  list-style: none;
  padding: 0;
  margin: 0;
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.note {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-left: 4px solid var(--accent);
  padding: 12px 16px;
  border-radius: var(--radius-md, 8px);
}
.note-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 6px;
}
.category-badge {
  display: inline-block;
  padding: 3px 10px;
  border-radius: 999px;
  color: white;
  font-size: 11px;
  font-weight: 700;
  letter-spacing: 0.3px;
}
.note-author {
  font-weight: 600;
  font-size: 13px;
  color: var(--text-primary);
}
.note-date {
  font-size: 11px;
  color: var(--text-secondary);
  margin-left: auto;
  font-family: "JetBrains Mono", monospace;
}
.note-content {
  white-space: pre-wrap;
  word-break: break-word;
  font-size: 13px;
  line-height: 1.5;
  color: var(--text-primary);
}

.loading,
.empty {
  padding: 24px;
  text-align: center;
  color: var(--text-secondary);
  font-size: 13px;
}
</style>
