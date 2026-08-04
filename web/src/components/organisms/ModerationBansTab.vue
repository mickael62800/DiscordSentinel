<script setup lang="ts">
import { ref } from "vue";
import { useToast } from "../../composables/useToast";
import { useBans } from "../../composables/useBans";
import { useConfirm } from "../../composables/useConfirm";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { useFormatDate } from "../../composables/useFormatDate";
import type { Infraction, ConfirmedBan } from "../../types";
import AppBadge from "../atoms/AppBadge.vue";
import LoadingState from "../atoms/LoadingState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import BanModal from "../molecules/BanModal.vue";

const { success, error: showError } = useToast();
const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();

const banModalRef = ref<InstanceType<typeof BanModal> | null>(null);
const unbanError = ref<string | null>(null);
const banModalVisible = ref(false);
const banModalTarget = ref<Infraction | null>(null);

const {
  filteredConfirmed,
  totalConfirmed,
  loading: bansLoading,
  banning,
  searchQuery: bansSearchQuery,
  executeBan,
  executeUnban,
  fetchBans,
} = useBans();
useRealtimeRefresh(["infraction_new", "moderation_action"], fetchBans);

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
  } catch (e) {
    unbanError.value = String(e);
  }
}

const bulkUnbanning = ref(false);
const unbanProgress = ref(0);
const unbanTotal = ref(0);

async function onUnbanAll() {
  unbanError.value = null;
  const targets = [...filteredConfirmed.value];
  if (targets.length === 0) return;

  const ok1 = await confirm({
    message:
      `⚠️ Tout débannir ⚠️\n\n` +
      `Vous allez débannir ${targets.length} utilisateur(s) sur Discord.\n\n` +
      `Chaque débannissement génère un appel à l'API Discord. ` +
      `Pour de grosses listes, le rate-limit peut ralentir l'opération.\n\n` +
      `Continuer ?`,
  });
  if (!ok1) return;
  const ok2 = await confirm({
    message: `Dernière confirmation : débannir ${targets.length} utilisateur(s) ?`,
  });
  if (!ok2) return;

  bulkUnbanning.value = true;
  unbanProgress.value = 0;
  unbanTotal.value = targets.length;
  let okCount = 0;
  let failCount = 0;
  const errors: string[] = [];

  for (const ban of targets) {
    try {
      await executeUnban(ban.guild_id, ban.target_id);
      okCount++;
    } catch (e) {
      failCount++;
      errors.push(`${ban.target_name}: ${String(e)}`);
    }
    unbanProgress.value++;
    await new Promise((r) => setTimeout(r, 80));
  }

  bulkUnbanning.value = false;

  if (failCount === 0) {
    success(`✅ ${okCount} utilisateur(s) débanni(s) avec succès.`);
  } else {
    showError(
      `${okCount} succès, ${failCount} échecs. ` +
        (errors.length > 0 ? `Premier échec : ${errors[0]}` : ""),
    );
  }
}
</script>

<template>
  <div>
    <div class="bans-toolbar">
      <input
        v-model="bansSearchQuery"
        type="text"
        placeholder="Rechercher par nom, ID ou raison..."
        class="search-input"
      />
      <button
        type="button"
        class="unban-all-btn"
        :disabled="bulkUnbanning || filteredConfirmed.length === 0"
        :title="filteredConfirmed.length === 0
          ? 'Aucun banni actif'
          : `Débannir les ${filteredConfirmed.length} bannis affichés (owner uniquement)`"
        @click="onUnbanAll"
      >
        {{ bulkUnbanning
          ? `Débannissement… (${unbanProgress}/${unbanTotal})`
          : `Tout débannir (${filteredConfirmed.length})` }}
      </button>
    </div>

    <p v-if="unbanError" class="ban-error">{{ unbanError }}</p>

    <LoadingState v-if="bansLoading" />

    <div v-else class="bans-list-single">
      <div class="column-header">
        <h2>Bannis actifs</h2>
        <span class="count-badge">{{ totalConfirmed }}</span>
      </div>

      <div class="ban-list">
        <div v-for="ban in filteredConfirmed" :key="ban.id" class="card ban-card confirmed">
          <div class="ban-user">
            <div class="user-avatar-placeholder confirmed-avatar">
              {{ (ban.target_display_name || ban.target_name).charAt(0).toUpperCase() }}
            </div>
            <div class="user-info">
              <strong v-if="ban.target_display_name" class="display-name">{{ ban.target_display_name }}</strong>
              <span class="username">@{{ ban.target_name }}</span>
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
            <button class="unban-btn" :disabled="banning" @click="handleUnban(ban)">
              {{ banning ? 'Debannissement...' : 'Debannir' }}
            </button>
          </div>
        </div>

        <EmptyState
          v-if="filteredConfirmed.length === 0"
          :message="bansSearchQuery ? 'Aucun compte banni correspondant' : 'Aucun compte banni'"
        />
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
.search-input {
  width: 100%;
  max-width: 400px;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 13px;
  outline: none;
  transition: border-color var(--transition-base);
}
.search-input:focus { border-color: var(--accent); }
.search-input::placeholder { color: var(--text-secondary); opacity: 0.6; }

.bans-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}
.bans-toolbar .search-input { flex: 1; min-width: 240px; }

.unban-all-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid color-mix(in srgb, var(--danger) 50%, var(--border));
  border-radius: var(--radius-md);
  padding: 8px 14px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease, background-color 0.25s ease, border-color 0.2s ease, transform 0.2s ease, box-shadow 0.25s ease;
  white-space: nowrap;
}
.unban-all-btn:hover:not(:disabled) {
  color: white;
  background-color: var(--danger);
  border-color: var(--danger);
  box-shadow: 0 4px 14px color-mix(in srgb, var(--danger) 30%, transparent);
  transform: translateY(-1px);
}
.unban-all-btn:active:not(:disabled) { transform: scale(0.97); }
.unban-all-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.bans-list-single { width: 100%; }
.bans-list-single .ban-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
}

.column-header { display: flex; align-items: center; gap: 10px; margin-bottom: 16px; }
.column-header h2 { font-size: 18px; font-weight: 600; margin: 0; }

.count-badge {
  font-size: 12px;
  font-weight: 600;
  background-color: var(--danger);
  color: white;
  padding: 2px 8px;
  border-radius: var(--radius-md);
}

.ban-list { display: flex; flex-direction: column; gap: 12px; }

.ban-card {
  opacity: 0;
  animation: ban-card-enter 0.4s ease-out forwards;
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1),
    border-color 0.25s ease, box-shadow 0.3s ease;
}
.ban-card:nth-child(1)  { animation-delay: 0.04s; }
.ban-card:nth-child(2)  { animation-delay: 0.08s; }
.ban-card:nth-child(3)  { animation-delay: 0.12s; }
.ban-card:nth-child(4)  { animation-delay: 0.16s; }
.ban-card:nth-child(5)  { animation-delay: 0.20s; }
.ban-card:nth-child(6)  { animation-delay: 0.24s; }
.ban-card:nth-child(7)  { animation-delay: 0.28s; }
.ban-card:nth-child(8)  { animation-delay: 0.32s; }
.ban-card:nth-child(n+9) { animation-delay: 0.36s; }

@keyframes ban-card-enter {
  0%   { opacity: 0; transform: translateY(8px); }
  100% { opacity: 1; transform: translateY(0); }
}

.ban-card.confirmed { border-left: 3px solid var(--danger); }
.ban-card.confirmed:hover {
  transform: translateY(-2px);
  border-left-color: var(--danger);
  box-shadow: 0 8px 22px color-mix(in srgb, var(--danger) 14%, transparent);
}

.ban-user { display: flex; align-items: center; gap: 12px; margin-bottom: 12px; }

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
.confirmed-avatar { background: linear-gradient(135deg, var(--danger), #ff6b6b); }

.user-info { display: flex; flex-direction: column; gap: 2px; min-width: 0; }
.display-name {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.display-name + .username { font-weight: 400; font-size: 12px; color: var(--text-secondary); }
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

.ban-details { display: flex; flex-direction: column; gap: 6px; }
.detail-row { display: flex; align-items: center; gap: 8px; font-size: 13px; }
.detail-label { color: var(--text-secondary); min-width: 80px; font-weight: 500; }
.reason { color: var(--text-primary); }

.ban-actions { margin-top: 12px; display: flex; justify-content: flex-end; }

.unban-btn {
  background-color: transparent;
  color: var(--accent, var(--success));
  border: 1px solid var(--accent, var(--success));
  border-radius: var(--radius-sm);
  padding: 6px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-base);
}
.unban-btn:hover:not(:disabled) {
  background-color: var(--accent, var(--success));
  color: white;
}
.unban-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.ban-error { color: var(--danger); font-size: 13px; margin-bottom: 12px; }

.mono { font-family: "JetBrains Mono", "Cascadia Code", monospace; font-size: 12px; }

@media (prefers-reduced-motion: reduce) {
  .ban-card { animation: none !important; opacity: 1; transform: none !important; }
  .ban-card:hover { transform: none; }
}
</style>
