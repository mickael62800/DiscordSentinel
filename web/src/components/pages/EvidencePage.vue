<script setup lang="ts">
import AppButton from "../atoms/AppButton.vue";
import { ref, computed, watch } from "vue";
import { useToast } from "@/composables/useToast";
import { useInfractions } from "@/composables/useInfractions";
import { useSharedUserLookup } from "@/composables/useSharedUserLookup";
import { evidenceService } from "@/services/moderationAdvancedService";
import type { EvidenceEntry } from "@/types/moderation-advanced";
import type { Infraction } from "@/types";
import AppTextarea from "@/components/atoms/AppTextarea.vue";
import { useFormatDate } from "@/composables/useFormatDate";
import { safeLinkUrl } from "@/utils/safeUrl";

interface Props {
  /** Quand true, cache l'en-tete et le champ de recherche (utilise dans le
   *  sous-onglet "Notes & Preuves" ou l'ID est saisi au niveau parent). */
  embedded?: boolean;
}
const props = defineProps<Props>();

const { success, error: showError } = useToast();

// Flow : user_id -> liste des actions de ce user -> selection d'une action -> preuves attachees
// L'ID user est partage avec NotesPage via useSharedUserLookup pour qu'un
// seul champ saisi en haut du sous-onglet "Notes & Preuves" alimente les 2.
const { sharedUserId: lookupUserId } = useSharedUserLookup();
const { infractions, loading: infractionsLoading, fetchInfractions } = useInfractions();

const selectedActionId = ref<string | null>(null);
const evidenceEntries = ref<EvidenceEntry[]>([]);
const evidenceLoading = ref(false);

// Filtre les infractions sur le user saisi, garde uniquement les actions
// effectivement appliquees (source == "action"), pas les detections automod.
const userActions = computed<Infraction[]>(() => {
  const uid = lookupUserId.value.trim();
  if (!uid) return [];
  return (infractions.value ?? []).filter(
    (i) => i.user_id === uid && i.source === "action"
  );
});

const selectedAction = computed<Infraction | null>(() => {
  if (!selectedActionId.value) return null;
  return userActions.value.find((a) => a.id === selectedActionId.value) ?? null;
});

const draft = ref({ url: "", description: "" });

async function searchUser() {
  if (!lookupUserId.value.trim()) return;
  selectedActionId.value = null;
  evidenceEntries.value = [];
  await fetchInfractions();
}

async function selectAction(actionId: string) {
  selectedActionId.value = actionId;
  evidenceLoading.value = true;
  try {
    evidenceEntries.value = await evidenceService.list(actionId);
  } catch (e) {
    console.error(e);
    showError("Erreur chargement preuves.");
    evidenceEntries.value = [];
  } finally {
    evidenceLoading.value = false;
  }
}

async function onAdd() {
  if (!selectedActionId.value || !draft.value.url.trim()) return;
  try {
    await evidenceService.add({
      action_id: selectedActionId.value,
      url: draft.value.url.trim(),
      description: draft.value.description.trim() || null,
      uploaded_by: "desktop",
      uploaded_by_name: "Desktop App",
    });
    draft.value.url = "";
    draft.value.description = "";
    success("Preuve ajoutée.");
    await selectAction(selectedActionId.value);
  } catch (e) {
    console.error(e);
    showError("Erreur lors de l'ajout.");
  }
}

const { formatDateTimeShort: formatDate } = useFormatDate();

// Reset si l'user change. En embedded, fetch automatiquement les actions.
watch(lookupUserId, async (id) => {
  selectedActionId.value = null;
  evidenceEntries.value = [];
  if (props.embedded && id.trim()) {
    await fetchInfractions();
  }
});
</script>

<template>
  <div class="page page--constrained">
    <header v-if="!props.embedded" class="page-header">
      <h1>📎 Preuves modération</h1>
      <p class="lede">
        Recherche un utilisateur, choisis une de ses actions de modération,
        joins-y une URL (screenshot, lien Discord, paste, etc.) avec
        description optionnelle.
      </p>
    </header>

    <!-- Étape 1 : recherche user (cachee en embedded, le parent gere) -->
    <section v-if="!props.embedded" class="card">
      <h2>1. Utilisateur ciblé</h2>
      <div class="lookup">
        <input
          v-model="lookupUserId"
          placeholder="ID Discord de l'utilisateur (ex: 123456789012345678)"
          @keyup.enter="searchUser"
        />
        <AppButton variant="secondary" :disabled="!lookupUserId.trim() || infractionsLoading" @click="searchUser">
          {{ infractionsLoading ? "Recherche…" : "Rechercher" }}
        </AppButton>
      </div>
    </section>

    <!-- Étape 2 : liste des actions de l'user -->
    <section v-if="lookupUserId.trim() && !infractionsLoading" class="card">
      <h2>2. Action de modération à documenter</h2>
      <div v-if="userActions.length === 0" class="empty">
        Aucune action appliquée trouvée pour cet utilisateur.
      </div>
      <table v-else class="table">
        <thead>
          <tr>
            <th>Date</th>
            <th>Type</th>
            <th>Raison</th>
            <th>Modérateur</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="a in userActions"
            :key="a.id"
            :class="{ 'row-selected': selectedActionId === a.id }"
          >
            <td>{{ formatDate(a.created_at) }}</td>
            <td><strong>{{ a.action ?? a.infraction_type }}</strong></td>
            <td>{{ a.reason }}</td>
            <td>{{ a.moderator }}</td>
            <td>
              <AppButton variant="secondary" @click="selectAction(a.id)">
                {{ selectedActionId === a.id ? "✓ Sélectionnée" : "Choisir" }}
              </AppButton>
            </td>
          </tr>
        </tbody>
      </table>
    </section>

    <!-- Étape 3 : ajouter preuve à l'action sélectionnée -->
    <section v-if="selectedAction" class="card">
      <h2>3. Joindre une preuve à cette action</h2>
      <p class="muted small">
        Action ciblée : <strong>{{ selectedAction.action ?? selectedAction.infraction_type }}</strong>
        — {{ selectedAction.reason }}
        <span class="mono"> (id : {{ selectedAction.id.slice(0, 8) }}…)</span>
      </p>
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
          <AppTextarea v-model="draft.description" :rows="2" />
        </label>
        <div class="actions">
          <AppButton variant="primary" type="submit" :disabled="!draft.url.trim()">
            Joindre
          </AppButton>
        </div>
      </form>
    </section>

    <!-- Liste des preuves de l'action sélectionnée -->
    <section v-if="selectedAction" class="card">
      <h2>Preuves attachées</h2>
      <div v-if="evidenceLoading" class="loading">Chargement…</div>
      <div v-else-if="evidenceEntries.length === 0" class="empty">
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
          <tr v-for="e in evidenceEntries" :key="e.id">
            <td>{{ formatDate(e.uploaded_at) }}</td>
            <td>
              <a
                v-if="safeLinkUrl(e.url)"
                :href="safeLinkUrl(e.url)!"
                target="_blank"
                rel="noopener noreferrer"
                >{{ e.url }}</a
              >
              <span v-else>{{ e.url }}</span>
            </td>
            <td>{{ e.description ?? "—" }}</td>
            <td>{{ e.uploaded_by_name }}</td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>

<style scoped>
@import "./_admin-page-shared.css";

.row-selected {
  background-color: color-mix(in srgb, var(--accent) 15%, transparent);
}
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
.mono { font-family: "JetBrains Mono", monospace; opacity: 0.7; }
</style>
