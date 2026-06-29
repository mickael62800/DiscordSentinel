<script setup lang="ts">
import { ref, watch } from "vue";
import { useToast } from "../../../composables/useToast";
import { useModeration } from "../../../composables/useModeration";
import { useGuildSelector } from "../../../composables/useGuildSelector";
import { useGuildMembers } from "../../../composables/useGuildMembers";
import type { GuildMember } from "../../../types";
import { safeImageUrl } from "../../../utils/safeUrl";

import AppInput from "../../atoms/AppInput.vue";
import AppSelect from "../../atoms/AppSelect.vue";
import AppTextarea from "../../atoms/AppTextarea.vue";
import AppButton from "../../atoms/AppButton.vue";
import AppModal from "../../atoms/AppModal.vue";
import FormField from "../../atoms/FormField.vue";

const props = defineProps<{ open: boolean }>();
const emit = defineEmits<{
  close: [];
  /** Une action a ete appliquee avec succes ; le parent doit rafraichir sa liste. */
  submitted: [];
}>();

const { success } = useToast();
const { selectedGuildId } = useGuildSelector();
const { submitting, logAction } = useModeration();
const { members, searchMembers } = useGuildMembers();

const actionGuildId = ref(selectedGuildId.value || "");
const actionTargetId = ref("");
const actionTargetName = ref("");
const actionType = ref("warn");
const actionReason = ref("");
const actionGravity = ref("medium");
const actionDuration = ref<number | undefined>(undefined);
const actionError = ref<string | null>(null);
const actionSearch = ref("");
const actionSuggestions = ref<GuildMember[]>([]);
const actionShowSuggestions = ref(false);

watch(selectedGuildId, (newId) => {
  if (newId) actionGuildId.value = newId;
});

watch(
  () => props.open,
  (isOpen) => {
    if (isOpen) actionError.value = null;
  },
);

function closeActionModal() {
  actionTargetId.value = "";
  actionTargetName.value = "";
  actionReason.value = "";
  actionDuration.value = undefined;
  actionSearch.value = "";
  actionSuggestions.value = [];
  actionShowSuggestions.value = false;
  actionError.value = null;
  emit("close");
}

function isSnowflake(s: string): boolean {
  return /^\d{17,20}$/.test(s.trim());
}

function onActionSearchInput() {
  const q = actionSearch.value.trim();

  if (isSnowflake(q)) {
    actionTargetId.value = q;
    const member = members.value?.find((m) => m.id === q);
    if (member) {
      actionTargetName.value = member.display_name || member.username;
    } else if (!actionTargetName.value) {
      actionTargetName.value = q;
    }
    actionSuggestions.value = [];
    actionShowSuggestions.value = false;
    return;
  }

  actionSuggestions.value = searchMembers(q);
  actionShowSuggestions.value = actionSuggestions.value.length > 0;
}

function selectActionMember(member: GuildMember) {
  actionTargetId.value = member.id;
  actionTargetName.value = member.display_name || member.username;
  actionSearch.value = member.display_name || member.username;
  actionShowSuggestions.value = false;
}

function onActionSearchBlur() {
  setTimeout(() => { actionShowSuggestions.value = false; }, 200);
}

async function handleActionSubmit() {
  if (!actionGuildId.value || !actionReason.value) {
    actionError.value = "L'ID du serveur et la raison sont requis.";
    return;
  }

  if (!actionTargetId.value.trim() && actionTargetName.value.trim()) {
    const name = actionTargetName.value.trim().toLowerCase();
    const match = members.value?.find(
      (m) =>
        m.username.toLowerCase() === name ||
        (m.display_name?.toLowerCase() ?? "") === name,
    );
    if (match) actionTargetId.value = match.id;
  }

  if (actionTargetId.value.trim() && !actionTargetName.value.trim()) {
    const member = members.value?.find((m) => m.id === actionTargetId.value.trim());
    actionTargetName.value = member
      ? (member.display_name || member.username)
      : actionTargetId.value.trim();
  }

  if (!actionTargetId.value.trim()) {
    actionError.value =
      "Cible introuvable. Saisis un ID Discord (17-20 chiffres) ou choisis un membre dans la liste.";
    return;
  }

  actionError.value = null;
  try {
    const result = await logAction({
      guildId: actionGuildId.value,
      channelId: "web-panel",
      moderatorId: "web-admin",
      moderatorName: "Web Admin",
      targetId: actionTargetId.value.trim(),
      targetName: actionTargetName.value.trim() || actionTargetId.value.trim(),
      actionType: actionType.value,
      reason: actionReason.value,
      gravity: actionGravity.value,
      duration: actionType.value === "mute" || actionType.value === "ban" ? actionDuration.value : undefined,
    });
    success(`${result.action_type} applique a ${result.target_name}`);
    emit("submitted");
    closeActionModal();
  } catch (e) {
    actionError.value = String(e);
  }
}
</script>

<template>
  <AppModal
    :visible="open"
    title="Nouvelle action de moderation"
    size="lg"
    @close="closeActionModal"
  >
    <form class="action-form" @submit.prevent="handleActionSubmit">
      <FormField label="ID du serveur">
        <AppInput v-model="actionGuildId" type="text" placeholder="ID du serveur" />
      </FormField>

      <FormField label="Utilisateur cible (nom OU ID Discord)">
        <div class="autocomplete-wrapper">
          <input
            v-model="actionSearch"
            type="text"
            placeholder="Tapez un pseudo, ou collez un ID Discord (17-20 chiffres)…"
            autocomplete="off"
            @input="onActionSearchInput"
            @focus="onActionSearchInput"
            @blur="onActionSearchBlur"
          />
          <div v-if="actionShowSuggestions" class="autocomplete-list">
            <div
              v-for="member in actionSuggestions"
              :key="member.id"
              class="autocomplete-item"
              @mousedown="selectActionMember(member)"
            >
              <img
                v-if="safeImageUrl(member.avatar_url)"
                :src="safeImageUrl(member.avatar_url) ?? ''"
                class="autocomplete-avatar"
              />
              <div v-else class="avatar-placeholder autocomplete-avatar-placeholder">
                {{ (member.display_name || member.username).charAt(0).toUpperCase() }}
              </div>
              <div class="autocomplete-info">
                <span class="autocomplete-name">{{ member.display_name || member.username }}</span>
                <span class="autocomplete-id">{{ member.id }}</span>
              </div>
            </div>
          </div>
        </div>
      </FormField>

      <div class="form-row two-col">
        <FormField label="ID cible">
          <AppInput v-model="actionTargetId" type="text" placeholder="Auto ou manuel" />
        </FormField>
        <FormField label="Nom cible">
          <AppInput v-model="actionTargetName" type="text" placeholder="Auto ou manuel" />
        </FormField>
      </div>

      <div class="form-row two-col">
        <FormField label="Action">
          <AppSelect v-model="actionType">
            <option value="warn">Avertissement</option>
            <option value="mute">Sourdine</option>
            <option value="ban">Bannissement</option>
          </AppSelect>
        </FormField>
        <FormField label="Gravite">
          <AppSelect v-model="actionGravity">
            <option value="low">Faible</option>
            <option value="medium">Moyen</option>
            <option value="high">Eleve</option>
            <option value="critical">Critique</option>
          </AppSelect>
        </FormField>
      </div>

      <FormField
        v-if="actionType === 'mute' || actionType === 'ban'"
        label="Duree (secondes) — vide = permanent"
      >
        <input
          v-model.number="actionDuration"
          type="number"
          placeholder="600 = 10min, 3600 = 1h"
          :min="0"
        />
      </FormField>

      <FormField label="Raison">
        <AppTextarea v-model="actionReason" :rows="3" placeholder="Pourquoi cette action ?" />
      </FormField>

      <p v-if="actionError" class="error-msg">{{ actionError }}</p>
    </form>

    <template #footer>
      <AppButton variant="secondary" @click="closeActionModal">Annuler</AppButton>
      <AppButton variant="primary" :disabled="submitting" @click="handleActionSubmit">
        {{ submitting ? "Application…" : `Appliquer ${actionType}` }}
      </AppButton>
    </template>
  </AppModal>
</template>

<style scoped>
/* ---- Action form ---- */
.action-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.form-row { display: flex; gap: 12px; }
.form-row.two-col > :deep(.form-field) { flex: 1; }

:deep(.form-field) input,
:deep(.form-field) select,
:deep(.form-field) textarea {
  width: 100%;
  background-color: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
}
:deep(.form-field) input:focus,
:deep(.form-field) select:focus,
:deep(.form-field) textarea:focus { border-color: var(--accent); }
:deep(.form-field) textarea { resize: vertical; }

.error-msg { color: var(--danger); font-size: 13px; }

.autocomplete-wrapper { position: relative; }
.autocomplete-list {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  margin-top: 4px;
  max-height: 240px;
  overflow-y: auto;
  z-index: 100;
  box-shadow: var(--shadow-md);
}
.autocomplete-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  cursor: pointer;
  transition: background var(--transition-fast);
}
.autocomplete-item:hover { background: var(--bg-hover); }
.autocomplete-avatar { width: 28px; height: 28px; border-radius: 50%; flex-shrink: 0; }
.autocomplete-avatar-placeholder { width: 28px; height: 28px; font-size: 12px; }
.autocomplete-info { display: flex; flex-direction: column; gap: 1px; min-width: 0; }
.autocomplete-name {
  font-size: 13px;
  font-weight: 600;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.autocomplete-id {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}
</style>
