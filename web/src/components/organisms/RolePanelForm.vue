<script setup lang="ts">
import IconButton from "../atoms/IconButton.vue";
import AppButton from "../atoms/AppButton.vue";
import AppSelect from "@/components/atoms/AppSelect.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import { reactive } from "vue";
import { useRouter } from "vue-router";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useRolePanels } from "@/composables/useRolePanels";
import { useToast } from "@/composables/useToast";
import type { CreateRolePanelEntryPayload } from "@/services/rolePanelsService";
import ChannelSelect from "@/components/atoms/ChannelSelect.vue";
import RoleSelect from "@/components/atoms/RoleSelect.vue";
import NumberInputWithUnit from "@/components/atoms/NumberInputWithUnit.vue";
import AppTextarea from "@/components/atoms/AppTextarea.vue";

const router = useRouter();
const { selectedGuildId } = useGuildSelector();
const { createPanel } = useRolePanels();
const { error: showError } = useToast();

const draft = reactive({
  channel_id: "",
  title: "",
  description: "",
  mode: "button",
  max_roles: null as number | null,
  entries: [] as CreateRolePanelEntryPayload[],
});

const STYLES = [
  { key: "primary", label: "Primary (bleu)" },
  { key: "secondary", label: "Secondary (gris)" },
  { key: "success", label: "Success (vert)" },
  { key: "danger", label: "Danger (rouge)" },
];

function addEntry() {
  draft.entries.push({
    role_id: "",
    role_name: "",
    emoji: "",
    label: "",
    style: "primary",
    position: draft.entries.length,
  });
}

function removeEntry(idx: number) {
  draft.entries.splice(idx, 1);
}

async function onSavePanel() {
  if (!selectedGuildId.value) {
    showError("Sélectionne une guild d'abord.");
    return;
  }
  if (!draft.title.trim() || !draft.channel_id.trim() || draft.entries.length === 0) {
    showError("Titre, salon et au moins 1 rôle requis.");
    return;
  }
  try {
    await createPanel({
      guild_id: selectedGuildId.value,
      channel_id: draft.channel_id.trim(),
      title: draft.title.trim(),
      description: draft.description.trim(),
      mode: draft.mode,
      max_roles: draft.max_roles,
      entries: draft.entries.map((e, idx) => ({ ...e, position: idx })),
    });
    router.push("/role-panels");
  } catch {
    /* toast deja affiche */
  }
}
</script>

<template>
  <section class="card">
    <h2>Configuration du panel</h2>
    <form @submit.prevent="onSavePanel" class="form">
      <label>Titre *
        <AppInput v-model="draft.title" required placeholder="Ex. Notifications" />
      </label>
      <label>Salon Discord *
        <ChannelSelect v-model="draft.channel_id" :guild-id="selectedGuildId" />
      </label>
      <label class="full">Description
        <AppTextarea v-model="draft.description" :rows="2" />
      </label>
      <label>Mode
        <AppSelect v-model="draft.mode">
          <option value="button">Boutons (jusqu'à 25 rôles)</option>
          <option value="select">Select menu (jusqu'à 25 rôles)</option>
        </AppSelect>
      </label>
      <label>Max rôles par user
        <NumberInputWithUnit v-model.number="draft.max_roles" :min="1" placeholder="vide = illimité" />
      </label>

      <div class="entries-section full">
        <div class="entries-header">
          <h3>Rôles ({{ draft.entries.length }})</h3>
          <AppButton variant="secondary" @click="addEntry">+ Ajouter un rôle</AppButton>
        </div>

        <div v-if="draft.entries.length === 0" class="empty">
          Aucun rôle ajouté. Clique « + Ajouter un rôle ».
        </div>

        <div v-else class="entry-list">
          <div v-for="(entry, idx) in draft.entries" :key="idx" class="entry-row">
            <RoleSelect v-model="entry.role_id" :guild-id="selectedGuildId" class="role-id" />
            <AppInput v-model="entry.role_name" placeholder="Nom (optionnel)" class="role-name" />
            <AppInput v-model="entry.emoji" placeholder="🎮" class="emoji" maxlength="4" />
            <AppInput v-model="entry.label" placeholder="Texte bouton" class="label" />
            <AppSelect v-model="entry.style" class="style">
              <option v-for="s in STYLES" :key="s.key" :value="s.key">{{ s.label }}</option>
            </AppSelect>
            <IconButton label="Supprimer" variant="danger" @click="removeEntry(idx)">🗑️</IconButton>
          </div>
        </div>
      </div>

      <div class="actions full">
        <router-link to="/role-panels" class="btn-secondary">Annuler</router-link>
        <AppButton variant="primary" type="submit">Créer le panel</AppButton>
      </div>
    </form>
  </section>
</template>

<style scoped>
@import "../pages/_admin-page-shared.css";
.form { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.form label { display: flex; flex-direction: column; gap: 4px; font-size: 0.9rem; }
.form label.full { grid-column: span 2; }
.form input, .form select, .form textarea {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
}
.entries-section { margin-top: 12px; }
.entries-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.entries-header h3 { margin: 0; font-size: 1rem; }
.entry-list { display: flex; flex-direction: column; gap: 8px; }
.entry-row {
  display: grid;
  grid-template-columns: 200px 150px 60px 1fr 180px 36px;
  gap: 6px;
  align-items: center;
}
.entry-row input, .entry-row select {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 4px 8px;
  color: inherit;
  font-size: 0.85rem;
}
.role-id { font-family: monospace; }
</style>
