<script setup lang="ts">
import { ref } from "vue";
import { useBans } from "../../composables/useBans";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { useConfirm } from "../../composables/useConfirm";
import AppBadge from "../atoms/AppBadge.vue";
import LoadingState from "../atoms/LoadingState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import type { Infraction, ConfirmedBan } from "../../types";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();
const banError = ref<string | null>(null);

// Modale de ban avec raison
const banModalVisible = ref(false);
const banModalTarget = ref<Infraction | null>(null);
const banModalReason = ref("");

const {
  filteredProposals,
  filteredConfirmed,
  totalProposals,
  totalConfirmed,
  loading,
  banning,
  searchQuery,
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
</script>

<template>
  <div class="bans">
    <h1>Comptes bannis</h1>

    <div class="filters">
      <input
        v-model="searchQuery"
        type="text"
        placeholder="Rechercher par nom, ID ou raison..."
        class="search-input"
      />
    </div>

    <p v-if="banError" class="ban-error">{{ banError }}</p>

    <LoadingState v-if="loading" />

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

          <EmptyState v-if="filteredConfirmed.length === 0" :message="searchQuery ? 'Aucun compte banni correspondant' : 'Aucun compte banni'" />
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

          <EmptyState v-if="filteredProposals.length === 0" :message="searchQuery ? 'Aucune proposition correspondante' : 'Aucune proposition'" />
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
</template>

<style scoped>
.bans h1 {
  margin-bottom: 24px;
}

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

/* Modale */
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
</style>
