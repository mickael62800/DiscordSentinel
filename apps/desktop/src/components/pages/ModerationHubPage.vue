<script setup lang="ts">
import { ref, watch } from "vue";

// --- Tab state ---
const activeTab = ref<"journal" | "bans" | "actions">("journal");

// --- Journal (infractions) ---
import { useInfractions } from "../../composables/useInfractions";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { useSearch } from "../../composables/useSearch";
import type { TableColumn, Infraction, ConfirmedBan, GuildMember } from "../../types";
import DataTable from "../organisms/DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";
import AppInput from "../atoms/AppInput.vue";
import LoadingState from "../atoms/LoadingState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import AppButton from "../atoms/AppButton.vue";
import FormField from "../atoms/FormField.vue";
import { infractionTypeVariant, actionVariant } from "../../utils/variants";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { infractions, loading: infractionsLoading, fetchInfractions } = useInfractions();
useRealtimeRefresh(["infraction_new"], fetchInfractions);
const { search: infractionsSearch, filtered: filteredInfractions } = useSearch<Infraction>(
  infractions,
  ["username", "user_id", "reason", "infraction_type", "moderator", "server", "created_at"],
);

const infractionsColumns: TableColumn[] = [
  { key: "username", label: "Utilisateur" },
  { key: "infraction_type", label: "Type" },
  { key: "reason", label: "Raison" },
  { key: "moderator", label: "Moderateur" },
  { key: "created_at", label: "Date" },
];

// --- Bans ---
import { useBans } from "../../composables/useBans";
import { useConfirm } from "../../composables/useConfirm";

const { confirm } = useConfirm();
const banError = ref<string | null>(null);

const banModalVisible = ref(false);
const banModalTarget = ref<Infraction | null>(null);
const banModalReason = ref("");

const {
  filteredProposals,
  filteredConfirmed,
  totalProposals,
  totalConfirmed,
  loading: bansLoading,
  banning,
  searchQuery: bansSearchQuery,
  executeBan,
  executeUnban,
  fetchBans,
} = useBans();
useRealtimeRefresh(["infraction_new", "moderation_action"], fetchBans);

function openBanModal(proposal: Infraction) {
  banModalTarget.value = proposal;
  banModalReason.value = proposal.reason || "";
  banModalVisible.value = true;
  banError.value = null;
}

function closeBanModal() {
  banModalVisible.value = false;
  banModalTarget.value = null;
  banModalReason.value = "";
}

async function confirmBan() {
  if (!banModalTarget.value) return;
  const proposal = banModalTarget.value;
  const reason = banModalReason.value.trim() || "Aucune raison specifiee";

  try {
    await executeBan(proposal.server, proposal.user_id, reason);
    closeBanModal();
  } catch (e) {
    banError.value = String(e);
  }
}

async function handleUnban(ban: ConfirmedBan) {
  banError.value = null;
  const ok = await confirm({ message: `Debannir ${ban.target_name} (${ban.target_id}) ?` });
  if (!ok) return;
  try {
    await executeUnban(ban.guild_id, ban.target_id);
  } catch (e) {
    banError.value = String(e);
  }
}

// --- Actions (moderation) ---
import { useModeration } from "../../composables/useModeration";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useGuildMembers } from "../../composables/useGuildMembers";

const { submitting, history, historyLoading, logAction, fetchHistory } = useModeration();
const { selectedGuildId } = useGuildSelector();
const { searchMembers } = useGuildMembers();

const guildId = ref(selectedGuildId.value || "");
const targetId = ref("");
const targetName = ref("");
const actionType = ref("warn");
const reason = ref("");
const gravity = ref("medium");
const duration = ref<number | undefined>(undefined);
const actionSuccess = ref<string | null>(null);
const actionError = ref<string | null>(null);

const lookupGuildId = ref(selectedGuildId.value || "");
const lookupUserId = ref("");

const targetSearch = ref("");
const suggestions = ref<GuildMember[]>([]);
const showSuggestions = ref(false);

function onTargetSearchInput() {
  suggestions.value = searchMembers(targetSearch.value);
  showSuggestions.value = suggestions.value.length > 0;
}

function selectMember(member: GuildMember) {
  targetId.value = member.id;
  targetName.value = member.display_name || member.username;
  targetSearch.value = member.display_name || member.username;
  showSuggestions.value = false;
}

function onTargetSearchBlur() {
  setTimeout(() => { showSuggestions.value = false; }, 200);
}

watch(selectedGuildId, (newId) => {
  if (newId) {
    guildId.value = newId;
    lookupGuildId.value = newId;
  }
});

async function handleSubmit() {
  if (!guildId.value || !targetId.value || !targetName.value || !reason.value) {
    actionError.value = "L'ID du serveur, l'ID de la cible, le nom de la cible et la raison sont requis.";
    return;
  }
  actionError.value = null;
  actionSuccess.value = null;
  try {
    const result = await logAction({
      guildId: guildId.value,
      channelId: "desktop-app",
      moderatorId: "desktop-admin",
      moderatorName: "Desktop Admin",
      targetId: targetId.value,
      targetName: targetName.value,
      actionType: actionType.value,
      reason: reason.value,
      gravity: gravity.value,
      duration: actionType.value === "mute" || actionType.value === "ban" ? duration.value : undefined,
    });
    actionSuccess.value = `${result.action_type} applique a ${result.target_name}`;
    targetId.value = "";
    targetName.value = "";
    reason.value = "";
    duration.value = undefined;
  } catch (e) {
    actionError.value = String(e);
  }
}

async function handleLookup() {
  if (!lookupGuildId.value || !lookupUserId.value) return;
  await fetchHistory(lookupGuildId.value, lookupUserId.value);
}
</script>

<template>
  <div class="moderation-hub">
    <h1>Moderation</h1>

    <!-- Tab bar -->
    <div class="hub-tabs">
      <button
        :class="['hub-tab', { active: activeTab === 'journal' }]"
        @click="activeTab = 'journal'"
      >
        Journal
      </button>
      <button
        :class="['hub-tab', { active: activeTab === 'bans' }]"
        @click="activeTab = 'bans'"
      >
        Bannissements
      </button>
      <button
        :class="['hub-tab', { active: activeTab === 'actions' }]"
        @click="activeTab = 'actions'"
      >
        Actions
      </button>
    </div>

    <!-- ============================================ -->
    <!-- TAB 1: Journal (infractions)                 -->
    <!-- ============================================ -->
    <div v-if="activeTab === 'journal'" class="tab-content">
      <div class="toolbar">
        <AppInput
          v-model="infractionsSearch"
          placeholder="Rechercher dans tous les champs..."
        />
      </div>

      <LoadingState v-if="infractionsLoading" />

      <DataTable
        v-else
        :columns="infractionsColumns"
        :rows="(filteredInfractions as unknown as Record<string, unknown>[])"
        empty-message="Aucune infraction"
      >
        <template #cell-username="{ row }">
          <div class="user-cell">
            <span class="username">{{ (row as Record<string, unknown>).username }}</span>
            <span class="user-id">{{ (row as Record<string, unknown>).user_id }}</span>
          </div>
        </template>
        <template #cell-infraction_type="{ value }">
          <AppBadge :label="String(value)" :variant="infractionTypeVariant(String(value))" />
        </template>
        <template #cell-created_at="{ value }">
          <span class="mono">{{ fmt(String(value)) }}</span>
        </template>
      </DataTable>
    </div>

    <!-- ============================================ -->
    <!-- TAB 2: Bannissements                         -->
    <!-- ============================================ -->
    <div v-if="activeTab === 'bans'" class="tab-content">
      <div class="filters">
        <input
          v-model="bansSearchQuery"
          type="text"
          placeholder="Rechercher par nom, ID ou raison..."
          class="search-input"
        />
      </div>

      <p v-if="banError" class="ban-error">{{ banError }}</p>

      <LoadingState v-if="bansLoading" />

      <div v-else class="bans-columns">
        <!-- Colonne gauche : Bannis effectifs -->
        <div class="bans-column">
          <div class="column-header">
            <h2>Bannis</h2>
            <span class="count-badge">{{ totalConfirmed }}</span>
          </div>

          <div class="ban-list">
            <div v-for="ban in filteredConfirmed" :key="ban.id" class="ban-card confirmed">
              <div class="ban-user">
                <div class="user-avatar-placeholder confirmed-avatar">{{ ban.target_name.charAt(0).toUpperCase() }}</div>
                <div class="user-info">
                  <span class="username">{{ ban.target_name }}</span>
                  <span class="user-id">{{ ban.target_id }}</span>
                </div>
              </div>
              <div class="ban-details">
                <div class="detail-row">
                  <span class="detail-label">Raison</span>
                  <span class="reason">{{ ban.reason }}</span>
                </div>
                <div class="detail-row">
                  <span class="detail-label">Banni par</span>
                  <AppBadge :label="ban.moderator_name" variant="info" />
                </div>
                <div class="detail-row">
                  <span class="detail-label">Type</span>
                  <AppBadge
                    :label="ban.action_type === 'ban_permanent' ? 'Permanent' : 'Temporaire'"
                    :variant="ban.action_type === 'ban_permanent' ? 'danger' : 'warning'"
                  />
                </div>
                <div class="detail-row">
                  <span class="detail-label">Date</span>
                  <span class="mono">{{ fmt(ban.created_at) }}</span>
                </div>
              </div>
              <div class="ban-actions">
                <button
                  class="unban-btn"
                  :disabled="banning"
                  @click="handleUnban(ban)"
                >
                  {{ banning ? 'Debannissement...' : 'Debannir' }}
                </button>
              </div>
            </div>

            <EmptyState v-if="filteredConfirmed.length === 0" :message="bansSearchQuery ? 'Aucun compte banni correspondant' : 'Aucun compte banni'" />
          </div>
        </div>

        <!-- Colonne droite : Propositions de ban -->
        <div class="bans-column">
          <div class="column-header">
            <h2>A bannir</h2>
            <span class="count-badge proposal">{{ totalProposals }}</span>
          </div>

          <div class="ban-list">
            <div v-for="proposal in filteredProposals" :key="proposal.id" class="ban-card proposal">
              <div class="ban-user">
                <div class="user-avatar-placeholder proposal-avatar">{{ proposal.username.charAt(0).toUpperCase() }}</div>
                <div class="user-info">
                  <span class="username">{{ proposal.username }}</span>
                  <span class="user-id">{{ proposal.user_id }}</span>
                </div>
              </div>
              <div class="ban-details">
                <div class="detail-row">
                  <span class="detail-label">Raison</span>
                  <span class="reason">{{ proposal.reason }}</span>
                </div>
                <div class="detail-row">
                  <span class="detail-label">Detecte par</span>
                  <AppBadge label="Automod" variant="warning" />
                </div>
                <div class="detail-row">
                  <span class="detail-label">Date</span>
                  <span class="mono">{{ fmt(proposal.created_at) }}</span>
                </div>
              </div>
              <div class="ban-actions">
                <button
                  class="ban-btn"
                  :disabled="banning"
                  @click="openBanModal(proposal)"
                >
                  {{ banning ? 'Bannissement...' : 'Bannir' }}
                </button>
              </div>
            </div>

            <EmptyState v-if="filteredProposals.length === 0" :message="bansSearchQuery ? 'Aucune proposition correspondante' : 'Aucune proposition'" />
          </div>
        </div>
      </div>

      <!-- Modale de bannissement avec raison -->
      <teleport to="body">
        <div v-if="banModalVisible" class="modal-overlay" @click.self="closeBanModal">
          <div class="modal-content">
            <div class="modal-header">
              <h3>Bannir un utilisateur</h3>
              <button class="modal-close" @click="closeBanModal">&times;</button>
            </div>

            <div class="modal-body" v-if="banModalTarget">
              <div class="modal-user">
                <div class="user-avatar-placeholder proposal-avatar">
                  {{ banModalTarget.username.charAt(0).toUpperCase() }}
                </div>
                <div class="user-info">
                  <span class="username">{{ banModalTarget.username }}</span>
                  <span class="user-id">{{ banModalTarget.user_id }}</span>
                </div>
              </div>

              <label class="modal-label">Raison du bannissement</label>
              <textarea
                v-model="banModalReason"
                class="modal-textarea"
                rows="3"
                placeholder="Indiquez la raison du bannissement..."
              ></textarea>

              <p v-if="banError" class="ban-error">{{ banError }}</p>
            </div>

            <div class="modal-footer">
              <button class="modal-cancel" @click="closeBanModal">Annuler</button>
              <button
                class="ban-btn"
                :disabled="banning || !banModalReason.trim()"
                @click="confirmBan"
              >
                {{ banning ? 'Bannissement...' : 'Confirmer le ban' }}
              </button>
            </div>
          </div>
        </div>
      </teleport>
    </div>

    <!-- ============================================ -->
    <!-- TAB 3: Actions (moderation)                  -->
    <!-- ============================================ -->
    <div v-if="activeTab === 'actions'" class="tab-content">
      <div class="mod-grid">
        <!-- Action form -->
        <div class="mod-card">
          <h2>Appliquer une action</h2>
          <form class="action-form" @submit.prevent="handleSubmit">
            <div class="form-row">
              <FormField label="ID du serveur">
                <input v-model="guildId" type="text" placeholder="ID du serveur" />
              </FormField>
            </div>
            <div class="form-row">
              <FormField label="Utilisateur cible">
                <div class="autocomplete-wrapper">
                  <input
                    v-model="targetSearch"
                    type="text"
                    placeholder="Rechercher un membre ou entrer un ID..."
                    @input="onTargetSearchInput"
                    @focus="onTargetSearchInput"
                    @blur="onTargetSearchBlur"
                    autocomplete="off"
                  />
                  <div v-if="showSuggestions" class="autocomplete-list">
                    <div
                      v-for="member in suggestions"
                      :key="member.id"
                      class="autocomplete-item"
                      @mousedown="selectMember(member)"
                    >
                      <img
                        v-if="member.avatar_url"
                        :src="member.avatar_url"
                        class="autocomplete-avatar"
                      />
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
              </FormField>
            </div>
            <div class="form-row two-col">
              <FormField label="ID de l'utilisateur cible">
                <input v-model="targetId" type="text" placeholder="ID auto-rempli ou saisie manuelle" />
              </FormField>
              <FormField label="Nom de l'utilisateur cible">
                <input v-model="targetName" type="text" placeholder="Auto-rempli ou saisie manuelle" />
              </FormField>
            </div>
            <div class="form-row two-col">
              <FormField label="Action">
                <select v-model="actionType">
                  <option value="warn">Avertissement</option>
                  <option value="mute">Sourdine</option>
                  <option value="ban">Bannissement</option>
                </select>
              </FormField>
              <FormField label="Gravite">
                <select v-model="gravity">
                  <option value="low">Faible</option>
                  <option value="medium">Moyen</option>
                  <option value="high">Eleve</option>
                  <option value="critical">Critique</option>
                </select>
              </FormField>
            </div>
            <div v-if="actionType === 'mute' || actionType === 'ban'" class="form-row">
              <FormField label="Duree (secondes) — laisser vide pour permanent">
                <input v-model.number="duration" type="number" placeholder="600 = 10min, 3600 = 1h" :min="0" />
              </FormField>
            </div>
            <FormField label="Raison">
              <textarea v-model="reason" rows="2" placeholder="Pourquoi cette action est-elle prise ?"></textarea>
            </FormField>

            <p v-if="actionError" class="error-msg">{{ actionError }}</p>
            <p v-if="actionSuccess" class="success-msg">{{ actionSuccess }}</p>

            <AppButton variant="primary" class="submit-btn" :disabled="submitting">
              {{ submitting ? "Application..." : `Appliquer ${actionType}` }}
            </AppButton>
          </form>
        </div>

        <!-- History lookup -->
        <div class="mod-card">
          <h2>Historique de l'utilisateur</h2>
          <div class="lookup-form">
            <div class="form-row two-col">
              <FormField label="ID du serveur">
                <input v-model="lookupGuildId" type="text" placeholder="ID du serveur" />
              </FormField>
              <FormField label="ID de l'utilisateur">
                <input v-model="lookupUserId" type="text" placeholder="ID utilisateur Discord" />
              </FormField>
            </div>
            <AppButton variant="primary" :disabled="historyLoading" @click="handleLookup">
              {{ historyLoading ? "Chargement..." : "Rechercher" }}
            </AppButton>
          </div>

          <div v-if="history" class="history-result">
            <div class="history-header">
              <span class="history-name">{{ history.target_name }}</span>
              <span class="history-id">{{ history.target_id }}</span>
            </div>
            <div class="history-stats">
              <div class="stat"><span class="stat-num info">{{ history.total_warns }}</span> avertissements</div>
              <div class="stat"><span class="stat-num warning">{{ history.total_mutes }}</span> sourdines</div>
              <div class="stat"><span class="stat-num danger">{{ history.total_bans }}</span> bannissements</div>
            </div>
            <div v-if="history.actions.length > 0" class="history-actions">
              <div v-for="action in history.actions" :key="action.id" class="history-action">
                <AppBadge :label="action.action_type" :variant="actionVariant(action.action_type)" />
                <span class="action-reason">{{ action.reason }}</span>
              </div>
            </div>
            <div v-else class="empty-small">Aucune action enregistree</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.moderation-hub h1 {
  margin-bottom: 24px;
}

/* ---- Tab bar ---- */
.hub-tabs {
  display: flex;
  gap: 0;
  border-bottom: 2px solid var(--border);
  margin-bottom: 24px;
}

.hub-tab {
  padding: 10px 24px;
  background: none;
  border: none;
  border-bottom: 2px solid transparent;
  margin-bottom: -2px;
  color: var(--text-secondary);
  font-weight: 600;
  font-size: 0.9rem;
  cursor: pointer;
  transition: color 0.15s, border-color 0.15s;
}

.hub-tab:hover {
  color: var(--text-primary);
}

.hub-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.tab-content {
  animation: fadeIn 0.15s ease;
}

@keyframes fadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

/* ---- Journal (infractions) ---- */
.toolbar {
  display: flex;
  margin-bottom: 16px;
}

.toolbar input {
  max-width: 360px;
}

.user-cell {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

.mono {
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  font-size: 12px;
}

/* ---- Bans ---- */
.filters {
  margin-bottom: 16px;
}

.search-input {
  width: 100%;
  max-width: 400px;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s;
}

.search-input:focus {
  border-color: var(--accent);
}

.search-input::placeholder {
  color: var(--text-secondary);
  opacity: 0.6;
}

.bans-columns {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
  align-items: start;
}

.column-header {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 16px;
}

.column-header h2 {
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.count-badge {
  font-size: 12px;
  font-weight: 600;
  background-color: var(--danger);
  color: white;
  padding: 2px 8px;
  border-radius: 10px;
}

.count-badge.proposal {
  background-color: var(--warning, #f59e0b);
}

.ban-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.ban-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
}

.ban-card.confirmed {
  border-left: 3px solid var(--danger);
}

.ban-card.proposal {
  border-left: 3px solid var(--warning, #f59e0b);
}

.ban-user {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.user-avatar-placeholder {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 14px;
  color: white;
  flex-shrink: 0;
}

.confirmed-avatar {
  background: linear-gradient(135deg, var(--danger), #ff6b6b);
}

.proposal-avatar {
  background: linear-gradient(135deg, var(--warning, #f59e0b), #fbbf24);
}

.user-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.username {
  font-weight: 600;
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.user-id {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.ban-details {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.detail-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.detail-label {
  color: var(--text-secondary);
  min-width: 80px;
  font-weight: 500;
}

.reason {
  color: var(--text-primary);
}

.ban-actions {
  margin-top: 12px;
  display: flex;
  justify-content: flex-end;
}

.ban-btn {
  background-color: var(--danger);
  color: white;
  border: none;
  border-radius: 6px;
  padding: 6px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity 0.2s;
}

.ban-btn:hover:not(:disabled) {
  opacity: 0.85;
}

.ban-btn:disabled,
.unban-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.unban-btn {
  background-color: transparent;
  color: var(--accent, #22c55e);
  border: 1px solid var(--accent, #22c55e);
  border-radius: 6px;
  padding: 6px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.2s;
}

.unban-btn:hover:not(:disabled) {
  background-color: var(--accent, #22c55e);
  color: white;
}

.ban-error {
  color: var(--danger);
  font-size: 13px;
  margin-bottom: 12px;
}

/* ---- Ban modal ---- */
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

.modal-header h3 {
  margin: 0;
  font-size: 16px;
  font-weight: 600;
}

.modal-close {
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 24px;
  cursor: pointer;
  line-height: 1;
}

.modal-close:hover {
  color: var(--text-primary);
}

.modal-body {
  padding: 20px;
}

.modal-user {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  padding: 12px;
  background: var(--bg-hover);
  border-radius: 8px;
}

.modal-label {
  display: block;
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 8px;
}

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
  transition: border-color 0.2s;
}

.modal-textarea:focus {
  border-color: var(--accent);
}

.modal-textarea::placeholder {
  color: var(--text-secondary);
  opacity: 0.6;
}

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
  font-weight: 500;
  cursor: pointer;
  transition: background 0.2s;
}

.modal-cancel:hover {
  background: var(--bg-hover);
}

/* ---- Actions (moderation) ---- */
.mod-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

@media (max-width: 1100px) {
  .mod-grid { grid-template-columns: 1fr; }
}

.mod-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 24px;
}

.mod-card h2 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 20px;
}

.action-form, .lookup-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.form-row { display: flex; gap: 12px; }
.form-row.two-col > :deep(.form-field) { flex: 1; }

:deep(.form-field) input, :deep(.form-field) select, :deep(.form-field) textarea {
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

:deep(.form-field) input:focus, :deep(.form-field) select:focus, :deep(.form-field) textarea:focus {
  border-color: var(--accent);
}

:deep(.form-field) textarea { resize: vertical; }

.error-msg { color: var(--danger); font-size: 13px; }
.success-msg { color: var(--success); font-size: 13px; }

.submit-btn { width: 100%; margin-top: 4px; }

/* History */
.history-result { margin-top: 20px; }

.history-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 12px;
}

.history-name { font-weight: 600; font-size: 16px; }
.history-id { font-size: 11px; color: var(--text-secondary); font-family: monospace; }

.history-stats {
  display: flex;
  gap: 16px;
  margin-bottom: 16px;
}

.stat {
  font-size: 13px;
  color: var(--text-secondary);
}

.stat-num {
  font-weight: 700;
  font-size: 18px;
  margin-right: 4px;
}

.stat-num.info { color: var(--info); }
.stat-num.warning { color: var(--warning); }
.stat-num.danger { color: var(--danger); }

.history-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.history-action {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background-color: var(--bg-primary);
  border-radius: 8px;
}

.action-reason {
  font-size: 13px;
  color: var(--text-secondary);
}

.empty-small {
  color: var(--text-secondary);
  font-size: 13px;
  text-align: center;
  padding: 16px;
}

/* Autocomplete */
.autocomplete-wrapper {
  position: relative;
}

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
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.3);
}

.autocomplete-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  cursor: pointer;
  transition: background 0.15s;
}

.autocomplete-item:hover {
  background: var(--bg-hover);
}

.autocomplete-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  flex-shrink: 0;
}

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

.autocomplete-info {
  display: flex;
  flex-direction: column;
  gap: 1px;
  min-width: 0;
}

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
