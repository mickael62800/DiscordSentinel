<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import AppSelect from "@/components/atoms/AppSelect.vue";
import { computed, ref } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useNotes } from "@/composables/useNotes";
import AppTextarea from "@/components/atoms/AppTextarea.vue";

const { guildIdFilter } = useGuildSelector();
const { lookupUserId, add } = useNotes();

const draft = ref({
  content: "",
  category: "general",
  authorId: "desktop",
  authorName: "Desktop App",
});

const CATEGORIES = [
  { key: "general", label: "Général" },
  { key: "warning", label: "Avertissement" },
  { key: "positive", label: "Positif" },
  { key: "context", label: "Contexte" },
];

const canSubmit = computed(
  () => !!guildIdFilter.value && !!lookupUserId.value.trim() && !!draft.value.content.trim(),
);

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
</script>

<template>
  <section class="card">
    <h2>Ajouter une note</h2>
    <form class="add-form" @submit.prevent="onAdd">
      <label>
        Catégorie
        <AppSelect v-model="draft.category">
          <option v-for="c in CATEGORIES" :key="c.key" :value="c.key">{{ c.label }}</option>
        </AppSelect>
      </label>
      <label class="full">
        Contenu
        <AppTextarea v-model="draft.content" :rows="3" required />
      </label>
      <div class="actions">
        <AppButton variant="primary" type="submit" :disabled="!canSubmit">Ajouter</AppButton>
      </div>
    </form>
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
.add-form { display: flex; flex-direction: column; gap: 14px; }
.add-form label {
  display: flex; flex-direction: column; gap: 6px;
  font-size: 11px; font-weight: 700;
  text-transform: uppercase; letter-spacing: 0.6px;
  color: var(--text-secondary);
}
.add-form input, .add-form select, .add-form textarea {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md, 8px);
  padding: 8px 12px;
  color: var(--text-primary);
  font-family: inherit; font-size: 13px; font-weight: 500;
  outline: none;
  transition: border-color .15s, box-shadow .15s;
}
.add-form input:focus, .add-form select:focus, .add-form textarea:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent);
}
.add-form label.full textarea {
  width: 100%; resize: vertical; min-height: 80px;
  font-weight: 500; text-transform: none; letter-spacing: 0;
  color: var(--text-primary);
}
.btn-primary {
  border: 1px solid transparent;
  border-radius: var(--radius-md, 8px);
  padding: 8px 18px;
  cursor: pointer;
  font-size: 13px; font-weight: 600;
  background: var(--accent); color: white;
  transition: all .15s;
}
.btn-primary:hover:not(:disabled) {
  background: color-mix(in srgb, var(--accent) 88%, white);
  box-shadow: 0 4px 14px color-mix(in srgb, var(--accent) 35%, transparent);
}
.btn-primary:disabled { opacity: .55; cursor: not-allowed; box-shadow: none; }
.actions { display: flex; justify-content: flex-end; }
</style>
