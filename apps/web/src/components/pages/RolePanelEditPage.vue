<script setup lang="ts">
import { reactive } from "vue";
import { useRouter } from "vue-router";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useRolePanels } from "@/composables/useRolePanels";
import { useToast } from "@/composables/useToast";
import type { CreateRolePanelEntryPayload } from "@/services/rolePanelsService";

const router = useRouter();
const { selectedGuildId } = useGuildSelector();
const { createPanel, addAutoRole, autoRoles, removeAutoRole } = useRolePanels();
const { error: showError } = useToast();

const draft = reactive({
  channel_id: "",
  title: "",
  description: "",
  mode: "button",
  max_roles: null as number | null,
  entries: [] as CreateRolePanelEntryPayload[],
});

const autoRoleDraft = reactive({
  role_id: "",
  role_name: "",
  delay_secs: 0,
});

const STYLES = [
  { key: "primary", label: "Primary (bleu)", color: "#5865F2" },
  { key: "secondary", label: "Secondary (gris)", color: "#808080" },
  { key: "success", label: "Success (vert)", color: "#2ECC71" },
  { key: "danger", label: "Danger (rouge)", color: "#E74C3C" },
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
      entries: draft.entries.map((e, idx) => ({
        ...e,
        position: idx,
      })),
    });
    router.push("/role-panels");
  } catch {
    // Toast déjà affiché par useRolePanels
  }
}

async function onAddAutoRole() {
  if (!selectedGuildId.value) return;
  if (!autoRoleDraft.role_id.trim()) {
    showError("Role ID requis.");
    return;
  }
  try {
    await addAutoRole({
      guild_id: selectedGuildId.value,
      role_id: autoRoleDraft.role_id.trim(),
      role_name: autoRoleDraft.role_name.trim(),
      delay_secs: autoRoleDraft.delay_secs,
    });
    autoRoleDraft.role_id = "";
    autoRoleDraft.role_name = "";
    autoRoleDraft.delay_secs = 0;
  } catch {
    // toast deja affiche
  }
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>🎨 Nouveau panel de rôles</h1>
      <p class="lede">
        Crée un panneau de sélection de rôles auto-réactif. Une fois créé, déploie-le
        sur Discord avec <code>/roles-panel deploy panel_id:&lt;UUID&gt;</code>.
      </p>
      <router-link to="/role-panels" class="back-link">← Retour aux panels</router-link>
    </header>

    <section class="card">
      <h2>Configuration du panel</h2>
      <form @submit.prevent="onSavePanel" class="form">
        <label>
          Titre *
          <input v-model="draft.title" required placeholder="Ex. Notifications" />
        </label>
        <label>
          ID du salon Discord *
          <input v-model="draft.channel_id" required placeholder="123456789012345678" />
        </label>
        <label class="full">
          Description
          <textarea v-model="draft.description" rows="2"></textarea>
        </label>
        <label>
          Mode
          <select v-model="draft.mode">
            <option value="button">Boutons (jusqu'à 25 rôles)</option>
            <option value="select">Select menu (jusqu'à 25 rôles)</option>
          </select>
        </label>
        <label>
          Max rôles par user
          <input v-model.number="draft.max_roles" type="number" min="1" placeholder="vide = illimité" />
        </label>

        <div class="entries-section full">
          <div class="entries-header">
            <h3>Rôles ({{ draft.entries.length }})</h3>
            <button type="button" class="btn-secondary" @click="addEntry">+ Ajouter un rôle</button>
          </div>

          <div v-if="draft.entries.length === 0" class="empty">
            Aucun rôle ajouté. Clique « + Ajouter un rôle ».
          </div>

          <div v-else class="entry-list">
            <div v-for="(entry, idx) in draft.entries" :key="idx" class="entry-row">
              <input v-model="entry.role_id" placeholder="Role ID" class="role-id" />
              <input v-model="entry.role_name" placeholder="Nom" class="role-name" />
              <input v-model="entry.emoji" placeholder="🎮" class="emoji" maxlength="4" />
              <input v-model="entry.label" placeholder="Texte bouton" class="label" />
              <select v-model="entry.style" class="style">
                <option v-for="s in STYLES" :key="s.key" :value="s.key">{{ s.label }}</option>
              </select>
              <button type="button" class="btn-icon-danger" @click="removeEntry(idx)">🗑️</button>
            </div>
          </div>
        </div>

        <div class="actions full">
          <router-link to="/role-panels" class="btn-secondary">Annuler</router-link>
          <button type="submit" class="btn-primary">Créer le panel</button>
        </div>
      </form>
    </section>

    <section class="card">
      <h2>Auto-roles à l'arrivée</h2>
      <p class="hint">
        Rôles attribués automatiquement à tout nouveau membre.
        Utile pour distinguer les nouveaux des membres validés.
      </p>

      <div v-if="autoRoles.length > 0" class="auto-roles-list">
        <div v-for="ar in autoRoles" :key="ar.id" class="auto-role-row">
          <span><strong>{{ ar.role_name || ar.role_id }}</strong></span>
          <span v-if="ar.delay_secs > 0" class="muted">délai {{ ar.delay_secs }}s</span>
          <span v-else class="muted">immédiat</span>
          <button
            class="btn-icon-danger"
            @click="removeAutoRole(selectedGuildId ?? '', ar.role_id)"
          >🗑️</button>
        </div>
      </div>

      <div class="auto-role-form">
        <input v-model="autoRoleDraft.role_id" placeholder="Role ID" />
        <input v-model="autoRoleDraft.role_name" placeholder="Nom du rôle" />
        <input v-model.number="autoRoleDraft.delay_secs" type="number" min="0" placeholder="Délai (s)" />
        <button class="btn-primary" @click="onAddAutoRole">Ajouter</button>
      </div>
    </section>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.back-link {
  display: inline-block;
  margin-top: 8px;
  color: var(--text-muted, #888);
  text-decoration: none;
  font-size: 0.9rem;
}
.back-link:hover {
  color: #5865F2;
}
.form {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.form label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.9rem;
}
.form label.full {
  grid-column: span 2;
}
.form input,
.form select,
.form textarea {
  background: var(--bg-input, #2a2a2a);
  border: 1px solid var(--border-color, #444);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
}
.entries-section {
  margin-top: 12px;
}
.entries-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.entries-header h3 {
  margin: 0;
  font-size: 1rem;
}
.entry-list {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.entry-row {
  display: grid;
  grid-template-columns: 200px 150px 60px 1fr 180px 36px;
  gap: 6px;
  align-items: center;
}
.entry-row input,
.entry-row select {
  background: var(--bg-input, #2a2a2a);
  border: 1px solid var(--border-color, #444);
  border-radius: 4px;
  padding: 4px 8px;
  color: inherit;
  font-size: 0.85rem;
}
.role-id { font-family: monospace; }
.btn-icon-danger {
  background: none;
  border: none;
  color: var(--danger, #E74C3C);
  cursor: pointer;
  font-size: 1rem;
  padding: 4px;
}
.hint {
  color: var(--text-muted, #888);
  font-size: 0.85rem;
  margin-bottom: 12px;
}
.auto-roles-list {
  display: flex;
  flex-direction: column;
  gap: 6px;
  margin-bottom: 16px;
}
.auto-role-row {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 12px;
  align-items: center;
  padding: 6px 12px;
  background: var(--bg-input, #181818);
  border-radius: 4px;
}
.auto-role-form {
  display: grid;
  grid-template-columns: 200px 150px 100px auto;
  gap: 6px;
  align-items: center;
}
.auto-role-form input {
  background: var(--bg-input, #2a2a2a);
  border: 1px solid var(--border-color, #444);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
}
</style>
