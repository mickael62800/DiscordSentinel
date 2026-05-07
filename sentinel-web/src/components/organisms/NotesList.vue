<script setup lang="ts">
import { useNotes } from "@/composables/useNotes";

const { notes, loading, remove } = useNotes();

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

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "2-digit", month: "2-digit", year: "numeric",
    hour: "2-digit", minute: "2-digit",
  });
}
</script>

<template>
  <section class="card">
    <h2>Notes existantes</h2>
    <div v-if="loading" class="loading">Chargement…</div>
    <div v-else-if="notes.length === 0" class="empty">
      Aucune note pour cet utilisateur.
    </div>
    <ul v-else class="notes-list">
      <li v-for="n in notes" :key="n.id" class="note">
        <div class="note-header">
          <span class="category-badge" :style="{ backgroundColor: categoryColor(n.category) }">
            {{ categoryLabel(n.category) }}
          </span>
          <span class="note-author">{{ n.author_name }}</span>
          <span class="note-date">{{ formatDate(n.created_at) }}</span>
          <button class="btn-icon" @click="remove(n.id)" title="Supprimer">🗑️</button>
        </div>
        <div class="note-content">{{ n.content }}</div>
      </li>
    </ul>
  </section>
</template>

<style scoped>
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg, 12px);
  padding: 20px;
  margin-bottom: 20px;
}
.card h2 { margin: 0 0 16px 0; font-size: 16px; font-weight: 700; }
.notes-list {
  list-style: none; padding: 0; margin: 0;
  display: flex; flex-direction: column; gap: 8px;
}
.note {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-left: 4px solid var(--accent);
  padding: 12px 16px;
  border-radius: var(--radius-md, 8px);
}
.note-header {
  display: flex; align-items: center; gap: 10px;
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
.note-author { font-weight: 600; font-size: 13px; color: var(--text-primary); }
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
.btn-icon {
  width: 30px; height: 30px;
  display: inline-flex; align-items: center; justify-content: center;
  background: transparent;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm, 6px);
  color: var(--text-secondary);
  cursor: pointer; font-size: 14px;
  transition: all .15s;
}
.btn-icon:hover {
  color: var(--danger);
  border-color: var(--danger);
  background: color-mix(in srgb, var(--danger) 10%, transparent);
}
.loading, .empty {
  padding: 24px; text-align: center;
  color: var(--text-secondary); font-size: 13px;
}
</style>
