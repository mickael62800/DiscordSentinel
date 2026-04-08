<script setup lang="ts">
import { ref } from "vue";
import { useBans } from "../../composables/useBans";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { useConfirm } from "../../composables/useConfirm";
import { useToast } from "../../composables/useToast";

const { success, error: showError } = useToast();
import AppBadge from "../atoms/AppBadge.vue";
import LoadingState from "../atoms/LoadingState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import BanModal from "../molecules/BanModal.vue";
import type { Infraction, ConfirmedBan } from "../../types";
import { useFormatDate } from "../../composables/useFormatDate";

const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();
const banModalRef = ref<InstanceType<typeof BanModal> | null>(null);
const unbanError = ref<string | null>(null);

const banModalVisible = ref(false);
const banModalTarget = ref<Infraction | null>(null);

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
  banModalVisible.value = true;
}

function closeBanModal() {
  banModalVisible.value = false;
  banModalTarget.value = null;
}

async function onBanConfirm(reason: string) {
  if (!banModalTarget.value) return;
  const proposal = banModalTarget.value;
  try {
    await executeBan(proposal.server, proposal.user_id, reason);
    closeBanModal();
    success("Utilisateur banni avec succes");
  } catch (e) {
    banModalRef.value?.setError(String(e));
  }
}

async function handleUnban(ban: ConfirmedBan) {
  unbanError.value = null;
  const ok = await confirm({ message: `Debannir ${ban.target_name} (${ban.target_id}) ?` });
  if (!ok) return;
  try {
    await executeUnban(ban.guild_id, ban.target_id);
    success("Utilisateur debanni avec succes");
  } catch (e) {
    unbanError.value = String(e);
    showError("Erreur lors du debannissement");
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

    <p v-if="unbanError" class="ban-error">{{ unbanError }}</p>

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

    <BanModal
      ref="banModalRef"
      :visible="banModalVisible"
      :target="banModalTarget"
      :banning="banning"
      @close="closeBanModal"
      @confirm="onBanConfirm"
    />
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

</style>
