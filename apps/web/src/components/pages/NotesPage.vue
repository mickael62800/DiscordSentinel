<script setup lang="ts">
import { watch } from "vue";
import { useNotes } from "@/composables/useNotes";
import NotesLookup from "../organisms/NotesLookup.vue";
import NotesAddForm from "../organisms/NotesAddForm.vue";
import NotesList from "../organisms/NotesList.vue";

interface Props {
  /** Quand true, cache le bloc de recherche (l'ID user vient du parent). */
  embedded?: boolean;
}
const props = defineProps<Props>();

const { lookupUserId, fetch } = useNotes();

// En mode embedded, fetch automatiquement quand l'ID partage change.
watch(lookupUserId, (id) => {
  if (props.embedded && id.trim()) void fetch(id);
});
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

    <NotesLookup v-if="!props.embedded" />
    <NotesAddForm v-if="lookupUserId" />
    <NotesList v-if="lookupUserId" />
  </div>
</template>

<style scoped>
.notes-page { max-width: 900px; margin: 0 auto; }
.page-header { margin-bottom: 24px; }
.page-header h1 { margin: 0 0 8px 0; font-size: 22px; }
.lede { color: var(--text-secondary); margin: 0; font-size: 13px; }
</style>
