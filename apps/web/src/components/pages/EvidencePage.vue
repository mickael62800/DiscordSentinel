<script setup lang="ts">
import { ref } from "vue";
import { useToast } from "@/composables/useToast";
import { evidenceService } from "@/services/moderationAdvancedService";
import type { EvidenceEntry } from "@/types/moderation-advanced";

const { success, error: showError } = useToast();

const lookupActionId = ref("");
const entries = ref<EvidenceEntry[]>([]);
const loading = ref(false);

const draft = ref({
  url: "",
  description: "",
});

async function fetchEvidence() {
  if (!lookupActionId.value.trim()) {
    entries.value = [];
    return;
  }
  loading.value = true;
  try {
    entries.value = await evidenceService.list(lookupActionId.value.trim());
  } catch (e) {
    console.error(e);
    showError("Erreur chargement preuves.");
    entries.value = [];
  } finally {
    loading.value = false;
  }
}

async function onAdd() {
  if (!lookupActionId.value.trim() || !draft.value.url.trim()) return;
  try {
    await evidenceService.add({
      action_id: lookupActionId.value.trim(),
      url: draft.value.url.trim(),
      description: draft.value.description.trim() || null,
      uploaded_by: "desktop",
      uploaded_by_name: "Desktop App",
    });
    draft.value.url = "";
    draft.value.description = "";
    success("Preuve ajoutée.");
    await fetchEvidence();
  } catch (e) {
    console.error(e);
    showError("Erreur lors de l'ajout.");
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR");
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>📎 Preuves modération</h1>
      <p class="lede">
        Joins URLs (screenshots, links, transcripts) à une action de modération
        existante. L'<code>action_id</code> est l'UUID renvoyé par
        <code>POST /api/moderation/actions</code> ou affiché dans le journal.
      </p>
    </header>

    <section class="card">
      <h2>Action ciblée</h2>
      <div class="lookup">
        <input
          v-model="lookupActionId"
          placeholder="UUID de l'action de modération"
          @keyup.enter="fetchEvidence"
        />
        <button class="btn-secondary" @click="fetchEvidence">Rechercher</button>
      </div>
    </section>

    <section v-if="lookupActionId" class="card">
      <h2>Ajouter une preuve</h2>
      <form class="add-form" @submit.prevent="onAdd">
        <label>
          URL (lien Discord, imgur, paste, etc.)
          <input
            v-model="draft.url"
            type="url"
            placeholder="https://..."
            required
          />
        </label>
        <label>
          Description (optionnel, max 500 chars)
          <textarea v-model="draft.description" rows="2"></textarea>
        </label>
        <div class="actions">
          <button type="submit" class="btn-primary" :disabled="!draft.url.trim()">
            Joindre
          </button>
        </div>
      </form>
    </section>

    <section v-if="lookupActionId" class="card">
      <h2>Preuves attachées</h2>
      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="entries.length === 0" class="empty">
        Aucune preuve attachée à cette action.
      </div>
      <table v-else class="table">
        <thead>
          <tr>
            <th>Date</th>
            <th>URL</th>
            <th>Description</th>
            <th>Auteur</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="e in entries" :key="e.id">
            <td>{{ formatDate(e.uploaded_at) }}</td>
            <td><a :href="e.url" target="_blank" rel="noopener">{{ e.url }}</a></td>
            <td>{{ e.description ?? "—" }}</td>
            <td>{{ e.uploaded_by_name }}</td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
</style>
