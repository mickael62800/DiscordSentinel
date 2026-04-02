<script setup lang="ts">
import { ref } from "vue";
import { useGuildMembers } from "../../composables/useGuildMembers";
import { API_BASE_URL } from "../../utils/api";
import type { GuildMember } from "../../types";

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

function closeModal() {
  emit("close");
}

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
    await fetch(`${API_BASE_URL}/api/watched-users`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        guild_id: props.guildId,
        user_id: addSelectedMember.value.id,
        username: addSelectedMember.value.display_name || addSelectedMember.value.username,
        reason: addReason.value,
      }),
    });
    closeModal();
    emit("added");
  } catch (e) {
    console.error("Erreur ajout surveillance:", e);
  } finally {
    addLoading.value = false;
  }
}
</script>

<template>
  <teleport to="body">
    <div v-if="visible" class="modal-overlay" @click.self="closeModal">
      <div class="modal-content">
        <div class="modal-header">
          <h3>Surveiller un membre</h3>
          <button class="modal-close" @click="closeModal">&times;</button>
        </div>

        <div class="modal-body">
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
                <img v-if="member.avatar_url" :src="member.avatar_url" class="autocomplete-avatar" />
                <div v-else class="autocomplete-avatar-placeholder">
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
            <img v-if="addSelectedMember.avatar_url" :src="addSelectedMember.avatar_url" class="selected-avatar" />
            <div v-else class="autocomplete-avatar-placeholder">
              {{ (addSelectedMember.display_name || addSelectedMember.username).charAt(0).toUpperCase() }}
            </div>
            <div>
              <strong>{{ addSelectedMember.display_name || addSelectedMember.username }}</strong>
              <div class="autocomplete-id">{{ addSelectedMember.id }}</div>
            </div>
          </div>

          <label class="modal-label" style="margin-top: 16px;">Raison de la surveillance</label>
          <textarea
            v-model="addReason"
            class="modal-textarea"
            rows="2"
            placeholder="Pourquoi surveiller ce membre ? (optionnel)"
          ></textarea>
        </div>

        <div class="modal-footer">
          <button class="modal-cancel" @click="closeModal">Annuler</button>
          <button
            class="add-confirm-btn"
            :disabled="!addSelectedMember || addLoading"
            @click="confirmAddWatch"
          >
            {{ addLoading ? 'Ajout...' : 'Mettre en surveillance' }}
          </button>
        </div>
      </div>
    </div>
  </teleport>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
}

.modal-content {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  width: 100%;
  max-width: 480px;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
}

.modal-header h3 { margin: 0; font-size: 16px; }
.modal-close { background: none; border: none; color: var(--text-secondary); font-size: 24px; cursor: pointer; }
.modal-close:hover { color: var(--text-primary); }

.modal-body { padding: 20px; }
.modal-label { display: block; font-size: 13px; font-weight: 600; color: var(--text-secondary); margin-bottom: 8px; }

.modal-input {
  width: 100%;
  background: var(--bg-input, var(--bg-card));
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  color: var(--text-primary);
  font-size: 14px;
  outline: none;
}

.modal-input:focus { border-color: var(--accent); }

.modal-textarea {
  width: 100%;
  background: var(--bg-input, var(--bg-card));
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 12px;
  color: var(--text-primary);
  font-size: 14px;
  font-family: inherit;
  resize: vertical;
  outline: none;
}

.modal-textarea:focus { border-color: var(--accent); }

.modal-footer {
  display: flex;
  justify-content: flex-end;
  gap: 10px;
  padding: 16px 20px;
  border-top: 1px solid var(--border);
}

.modal-cancel {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 16px;
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
}

.modal-cancel:hover { background: var(--bg-hover); }

.add-confirm-btn {
  background: var(--accent);
  color: white;
  border: none;
  border-radius: 6px;
  padding: 8px 20px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
}

.add-confirm-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.selected-member {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-top: 12px;
  padding: 12px;
  background: var(--bg-hover);
  border-radius: 8px;
}

.selected-avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
}

/* Autocomplete */
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
  max-height: 200px;
  overflow-y: auto;
  z-index: 1001;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.autocomplete-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  cursor: pointer;
}

.autocomplete-item:hover { background: var(--bg-hover); }
.autocomplete-avatar { width: 28px; height: 28px; border-radius: 50%; flex-shrink: 0; }

.autocomplete-avatar-placeholder {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--accent), #6366f1);
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 12px;
  font-weight: 700;
  color: white;
  flex-shrink: 0;
}

.autocomplete-info { display: flex; flex-direction: column; gap: 1px; }
.autocomplete-name { font-size: 13px; font-weight: 600; }
.autocomplete-id { font-size: 11px; color: var(--text-secondary); font-family: monospace; }
</style>
