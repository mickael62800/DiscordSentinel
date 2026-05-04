<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { tempRolesService } from "@/services/polishServices";
import type { TempRole } from "@/types/polish";

const { guildIdFilter } = useGuildSelector();
const { success, error: showError } = useToast();
const tempRoles = ref<TempRole[]>([]);
const loading = ref(true);

const draft = reactive({
  user_id: "",
  role_id: "",
  duration_hours: 24,
});

async function fetchTempRoles() {
  if (!guildIdFilter.value) {
    tempRoles.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    tempRoles.value = await tempRolesService.list(guildIdFilter.value);
  } catch (e) {
    console.error(e);
    showError("Erreur chargement temp-roles.");
  } finally {
    loading.value = false;
  }
}

async function onCreate() {
  if (!guildIdFilter.value) return;
  if (!draft.user_id.trim() || !draft.role_id.trim()) {
    showError("User ID + Role ID requis.");
    return;
  }
  try {
    const expires_at = new Date(
      Date.now() + draft.duration_hours * 3600 * 1000,
    ).toISOString();
    await tempRolesService.create({
      guild_id: guildIdFilter.value,
      user_id: draft.user_id.trim(),
      role_id: draft.role_id.trim(),
      expires_at,
    });
    draft.user_id = "";
    draft.role_id = "";
    success("Rôle temporaire créé.");
    await fetchTempRoles();
  } catch (e) {
    console.error(e);
    showError("Erreur création.");
  }
}

async function onDelete(t: TempRole) {
  if (!guildIdFilter.value) return;
  if (!confirm(`Retirer le rôle ${t.role_id} de l'utilisateur ${t.user_id} ?`)) return;
  try {
    await tempRolesService.remove(guildIdFilter.value, t.user_id, t.role_id);
    tempRoles.value = tempRoles.value.filter((r) => r.id !== t.id);
    success("Rôle temporaire supprimé.");
  } catch (e) {
    console.error(e);
    showError("Erreur suppression.");
  }
}

onMounted(fetchTempRoles);
watch(guildIdFilter, fetchTempRoles);

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR");
}

function timeRemaining(iso: string): string {
  const ms = new Date(iso).getTime() - Date.now();
  if (ms < 0) return "expiré";
  const h = Math.floor(ms / 3600000);
  if (h < 1) return `${Math.floor(ms / 60000)} min`;
  if (h < 24) return `${h} h`;
  return `${Math.floor(h / 24)} j`;
}

const sortedRoles = computed(() =>
  [...tempRoles.value].sort(
    (a, b) => new Date(a.expires_at).getTime() - new Date(b.expires_at).getTime(),
  ),
);
</script>

<template>
  <div class="page page--constrained">
    <header class="page-header">
      <h1>⏳ Rôles temporaires</h1>
      <p class="lede">
        Rôles attribués pour une durée limitée (ex. abonnement Premium 30j,
        promotion mod test 7j…). Le worker temp-roles supprime automatiquement
        le rôle Discord à expiration.
      </p>
    </header>

    <section class="card">
      <h2>Nouveau rôle temporaire</h2>
      <form @submit.prevent="onCreate" class="form">
        <label>
          User ID *
          <input v-model="draft.user_id" required />
        </label>
        <label>
          Role ID *
          <input v-model="draft.role_id" required />
        </label>
        <label>
          Durée (heures) *
          <input v-model.number="draft.duration_hours" type="number" min="1" required />
        </label>
        <div class="actions full">
          <button type="submit" class="btn-primary">Créer</button>
        </div>
      </form>
    </section>

    <section class="card">
      <h2>Rôles actifs ({{ tempRoles.length }})</h2>
      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="tempRoles.length === 0" class="empty">
        Aucun rôle temporaire actif.
      </div>
      <table v-else class="table">
        <thead>
          <tr>
            <th>User</th>
            <th>Rôle</th>
            <th>Expire</th>
            <th>Restant</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="t in sortedRoles" :key="t.id">
            <td><code>{{ t.user_id }}</code></td>
            <td><code>{{ t.role_id }}</code></td>
            <td>{{ formatDate(t.expires_at) }}</td>
            <td><strong>{{ timeRemaining(t.expires_at) }}</strong></td>
            <td>
              <button class="btn-icon-danger" @click="onDelete(t)">🗑️</button>
            </td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.form {
  display: grid;
  grid-template-columns: 1fr;
  gap: 12px;
}
.form label {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.form input {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  color: inherit;
  font-size: 13px;
}
.actions.full {
  justify-content: flex-end;
}
.btn-icon-danger {
  background: none;
  border: none;
  color: var(--danger, #E74C3C);
  cursor: pointer;
  font-size: 1rem;
}
</style>
