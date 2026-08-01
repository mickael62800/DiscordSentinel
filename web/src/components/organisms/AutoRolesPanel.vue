<script setup lang="ts">
import IconButton from "../atoms/IconButton.vue";
import AppButton from "../atoms/AppButton.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import { reactive } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useRolePanels } from "@/composables/useRolePanels";
import { useToast } from "@/composables/useToast";
import RoleSelect from "@/components/atoms/RoleSelect.vue";
import NumberInputWithUnit from "@/components/atoms/NumberInputWithUnit.vue";

const { selectedGuildId } = useGuildSelector();
const { autoRoles, addAutoRole, removeAutoRole } = useRolePanels();
const { error: showError } = useToast();

const draft = reactive({
  role_id: "",
  role_name: "",
  delay_secs: 0,
});

async function onAdd() {
  if (!selectedGuildId.value) return;
  if (!draft.role_id.trim()) {
    showError("Role ID requis.");
    return;
  }
  try {
    await addAutoRole({
      guild_id: selectedGuildId.value,
      role_id: draft.role_id.trim(),
      role_name: draft.role_name.trim(),
      delay_secs: draft.delay_secs,
    });
    draft.role_id = "";
    draft.role_name = "";
    draft.delay_secs = 0;
  } catch {
    /* toast deja affiche */
  }
}
</script>

<template>
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
        <IconButton label="Supprimer" variant="danger" @click="removeAutoRole(selectedGuildId ?? '', ar.role_id)">🗑️</IconButton>
      </div>
    </div>

    <div class="auto-role-form">
      <RoleSelect v-model="draft.role_id" :guild-id="selectedGuildId" />
      <AppInput v-model="draft.role_name" placeholder="Nom (optionnel)" />
      <NumberInputWithUnit v-model.number="draft.delay_secs" :min="0" unit="s" placeholder="Délai" />
      <AppButton variant="primary" @click="onAdd">Ajouter</AppButton>
    </div>
  </section>
</template>

<style scoped>
@import "../pages/_admin-page-shared.css";
.hint { color: var(--text-secondary); font-size: 0.85rem; margin-bottom: 12px; }
.auto-roles-list {
  display: flex; flex-direction: column;
  gap: 6px; margin-bottom: 16px;
}
.auto-role-row {
  display: grid;
  grid-template-columns: 1fr auto auto;
  gap: 12px;
  align-items: center;
  padding: 6px 12px;
  background: var(--bg-card);
  border-radius: var(--radius-sm);
}
.auto-role-form {
  display: grid;
  grid-template-columns: 200px 150px 100px auto;
  gap: 6px;
  align-items: center;
}
.auto-role-form input {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 6px 10px;
  color: inherit;
}
</style>
