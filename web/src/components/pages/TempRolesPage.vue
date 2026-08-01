<script setup lang="ts">
import IconButton from "../atoms/IconButton.vue";
import AppButton from "../atoms/AppButton.vue";
import AppInput from "@/components/atoms/AppInput.vue";
import { computed, onMounted, reactive, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import { tempRolesService } from "@/services/polishServices";
import type { TempRole } from "@/types/polish";
import RoleSelect from "@/components/atoms/RoleSelect.vue";
import NumberInputWithUnit from "@/components/atoms/NumberInputWithUnit.vue";
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";
import { useFormatDate } from "@/composables/useFormatDate";

const { guildIdFilter } = useGuildSelector();
const { formatDateTimeShort: formatDate } = useFormatDate();
const { success, error: showError } = useToast();
const { confirm } = useConfirm();
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
  if (
    !(await confirm({
      title: "Retirer le rôle temporaire",
      message: `Retirer le rôle ${t.role_id} de l'utilisateur ${t.user_id} ?`,
    }))
  )
    return;
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
  <AdminPageShell title="Rôles temporaires" icon="⏳">
    <template #lede>
      Rôles attribués pour une durée limitée (ex. abonnement Premium 30j,
      promotion mod test 7j…). Le worker temp-roles supprime automatiquement
      le rôle Discord à expiration.
    </template>

    <section class="card">
      <h2>Nouveau rôle temporaire</h2>
      <form @submit.prevent="onCreate" class="form">
        <label>
          User ID *
          <AppInput v-model="draft.user_id" required />
        </label>
        <label>
          Rôle *
          <RoleSelect v-model="draft.role_id" :guild-id="guildIdFilter ?? null" />
        </label>
        <label>
          Durée (heures) *
          <NumberInputWithUnit v-model.number="draft.duration_hours" :min="1" required unit="h" />
        </label>
        <div class="actions full">
          <AppButton variant="primary" type="submit">Créer</AppButton>
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
              <IconButton label="Supprimer" variant="danger" @click="onDelete(t)">🗑️</IconButton>
            </td>
          </tr>
        </tbody>
      </table>
    </section>
  </AdminPageShell>
</template>

<style scoped>
@import "./_admin-page-shared.css";
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
  border-radius: var(--radius-sm);
  padding: 8px 12px;
  color: inherit;
  font-size: 13px;
}
.actions.full {
  justify-content: flex-end;
}
</style>
