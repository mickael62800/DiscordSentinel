<script setup lang="ts">
import { ref } from "vue";
import { useGuildMembers } from "../../composables/useGuildMembers";
import { useToast } from "../../composables/useToast";
import { getApiBaseUrl } from "../../utils/api";
import { safeImageUrl } from "../../utils/safeUrl";
import type { GuildMember } from "../../types";
import AppModal from "../atoms/AppModal.vue";
import AppButton from "../atoms/AppButton.vue";

const { success, error: showError } = useToast();

const props = defineProps<{
  visible: boolean;
  guildId: string;
}>();

const emit = defineEmits<{
  close: [];
  added: [];
}>();

const { searchMembers } = useGuildMembers();

const addSearch = ref("");
const addSuggestions = ref<GuildMember[]>([]);
const showAddSuggestions = ref(false);
const addSelectedMember = ref<GuildMember | null>(null);
const addReason = ref("");
const addLoading = ref(false);

function openReset() {
  addSearch.value = "";
  addSelectedMember.value = null;
  addReason.value = "";
}

defineExpose({ openReset });

function closeModal() { emit("close"); }

function onAddSearchInput() {
  addSuggestions.value = searchMembers(addSearch.value);
  showAddSuggestions.value = addSuggestions.value.length > 0;
}

function selectAddMember(member: GuildMember) {
  addSelectedMember.value = member;
  addSearch.value = member.display_name || member.username;
  showAddSuggestions.value = false;
}

function onAddSearchBlur() {
  setTimeout(() => { showAddSuggestions.value = false; }, 200);
}

async function confirmAddWatch() {
  if (!addSelectedMember.value || !props.guildId) return;
  addLoading.value = true;
  try {
    const baseUrl = await getApiBaseUrl();
    await fetch(`${baseUrl}/api/watched-users`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        guild_id: props.guildId,
        user_id: addSelectedMember.value.id,
        username: addSelectedMember.value.display_name || addSelectedMember.value.username,
        reason: addReason.value,
      }),
    });
    success("Membre mis en surveillance avec succes");
    closeModal();
    emit("added");
  } catch (e) {
    console.error("Erreur ajout surveillance:", e);
    showError("Erreur lors de l'ajout en surveillance");
  } finally {
    addLoading.value = false;
  }
}
</script>

<template>
  <AppModal :visible="visible" title="Surveiller un membre" size="md" @close="closeModal">
    <label class="modal-label">Rechercher un membre</label>
    <div class="autocomplete-wrapper">
      <input
        v-model="addSearch"
        type="text"
        placeholder="Tapez le nom d'un membre..."
        class="modal-input"
        @input="onAddSearchInput"
        @focus="onAddSearchInput"
        @blur="onAddSearchBlur"
        autocomplete="off"
      />
      <div v-if="showAddSuggestions" class="autocomplete-list">
        <div
          v-for="member in addSuggestions"
          :key="member.id"
          class="autocomplete-item"
          @mousedown="selectAddMember(member)"
        >
          <img v-if="safeImageUrl(member.avatar_url)" :src="safeImageUrl(member.avatar_url) ?? ''" class="autocomplete-avatar" />
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

    <div v-if="addSelectedMember" class="selected-member">
      <img v-if="safeImageUrl(addSelectedMember.avatar_url)" :src="safeImageUrl(addSelectedMember.avatar_url) ?? ''" class="selected-avatar" />
      <div v-else class="avatar-placeholder autocomplete-avatar-placeholder">
        {{ (addSelectedMember.display_name || addSelectedMember.username).charAt(0).toUpperCase() }}
      </div>
      <div>
        <strong>{{ addSelectedMember.display_name || addSelectedMember.username }}</strong>
        <div class="autocomplete-id">{{ addSelectedMember.id }}</div>
      </div>
    </div>

    <label class="modal-label modal-label--spaced">Raison de la surveillance</label>
    <textarea
      v-model="addReason"
      class="modal-textarea"
      rows="2"
      placeholder="Pourquoi surveiller ce membre ? (optionnel)"
    ></textarea>

    <template #footer>
      <AppButton variant="secondary" @click="closeModal">Annuler</AppButton>
      <AppButton
        variant="primary"
        :disabled="!addSelectedMember || addLoading"
        @click="confirmAddWatch"
      >
        {{ addLoading ? 'Ajout...' : 'Mettre en surveillance' }}
      </AppButton>
    </template>
  </AppModal>
</template>

<style scoped>
.modal-label { display: block; font-size: 13px; font-weight: 600; color: var(--text-secondary); margin-bottom: var(--space-sm); }
.modal-label--spaced { margin-top: var(--space-lg); }

.modal-input {
  width: 100%;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
}
.modal-input:focus { border-color: var(--accent); }

.modal-textarea {
  width: 100%;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 10px var(--space-md);
  color: var(--text-primary);
  font-size: 14px;
  font-family: inherit;
  resize: vertical;
  outline: none;
}
.modal-textarea:focus { border-color: var(--accent); }

.selected-member {
  display: flex;
  align-items: center;
  gap: var(--space-md);
  margin-top: var(--space-md);
  padding: var(--space-md);
  background: var(--bg-hover);
  border-radius: var(--radius-md);
}

.selected-avatar { width: 36px; height: 36px; border-radius: 50%; }

.autocomplete-wrapper { position: relative; }
.autocomplete-list {
  position: absolute;
  top: 100%;
  left: 0;
  right: 0;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  margin-top: var(--space-xs);
  max-height: 200px;
  overflow-y: auto;
  z-index: 1001;
  box-shadow: var(--shadow-lg);
}
.autocomplete-item {
  display: flex;
  align-items: center;
  gap: var(--space-sm);
  padding: var(--space-sm) var(--space-md);
  cursor: pointer;
}
.autocomplete-item:hover { background: var(--bg-hover); }
.autocomplete-avatar { width: 28px; height: 28px; border-radius: 50%; flex-shrink: 0; }
.autocomplete-avatar-placeholder { width: 28px; height: 28px; font-size: 12px; }
.autocomplete-info { display: flex; flex-direction: column; gap: 1px; }
.autocomplete-name { font-size: 13px; font-weight: 600; }
.autocomplete-id { font-size: 11px; color: var(--text-secondary); font-family: monospace; }
</style>
