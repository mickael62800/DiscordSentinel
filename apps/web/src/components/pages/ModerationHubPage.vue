<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useToast } from "../../composables/useToast";
import { useInfractions } from "../../composables/useInfractions";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import type { TableColumn, Infraction, ConfirmedBan, GuildMember } from "../../types";
import DataTable from "../organisms/DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";
import AppInput from "../atoms/AppInput.vue";
import AppSelect from "../atoms/AppSelect.vue";
import LoadingState from "../atoms/LoadingState.vue";
import BanModal from "../molecules/BanModal.vue";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import AppButton from "../atoms/AppButton.vue";
import FormField from "../atoms/FormField.vue";
import { infractionTypeVariant } from "../../utils/variants";
import { useFormatDate } from "../../composables/useFormatDate";
import { useBans } from "../../composables/useBans";
import { useConfirm } from "../../composables/useConfirm";
import { useModeration } from "../../composables/useModeration";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useGuildMembers } from "../../composables/useGuildMembers";

const { success, error: showError } = useToast();
const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();

// --- Tabs ---
const activeTab = ref<"journal" | "bans">("journal");

// --- Journal : donnees + filtres ---
const {
  infractions,
  loading: infractionsLoading,
  error: infractionsError,
  fetchInfractions,
  deleting,
  deleteInfraction,
  purging,
  purgeAll,
} = useInfractions();
useRealtimeRefresh(["infraction_new", "strike_added", "conduct_points_changed"], fetchInfractions);

const journalSearch = ref("");
const journalType = ref<string>("all");
const journalModerator = ref<string>("all");
const journalDateFrom = ref<string>("");
const journalDateTo = ref<string>("");
// Les detections AutoMod sans sanction ("none" / "") polluent le journal :
// masquees par defaut, decochable pour tout voir.
const hideDetections = ref(true);

function isDetection(type: string | null | undefined): boolean {
  const t = String(type ?? "").toLowerCase();
  return t === "" || t === "none" || t === "detection";
}

function infractionTypeLabel(type: string | null | undefined): string {
  return isDetection(type) ? "Detection" : String(type);
}

// Listes distinctes derivees des infractions pour peupler les selects
const moderatorOptions = computed(() => {
  const set = new Set<string>();
  for (const inf of infractions.value ?? []) {
    if (inf.moderator) set.add(inf.moderator);
  }
  return [
    { value: "all", label: "Tous les moderateurs" },
    ...Array.from(set).sort().map((m) => ({ value: m, label: m })),
  ];
});

const typeOptions = computed(() => {
  const set = new Set<string>();
  for (const inf of infractions.value ?? []) {
    if (inf.infraction_type) set.add(inf.infraction_type);
  }
  return [
    { value: "all", label: "Tous les types" },
    ...Array.from(set).sort().map((t) => ({ value: t, label: t })),
  ];
});

const filteredInfractions = computed<Infraction[]>(() => {
  let rows = (infractions.value ?? []).slice();

  // Masquage des detections AutoMod sans sanction
  if (hideDetections.value) {
    rows = rows.filter((i) => !isDetection(i.infraction_type));
  }

  // Filtre texte
  const q = journalSearch.value.trim().toLowerCase();
  if (q) {
    rows = rows.filter((i) =>
      [i.username, i.user_id, i.reason, i.infraction_type, i.moderator, i.server]
        .some((f) => String(f ?? "").toLowerCase().includes(q)),
    );
  }

  // Filtre type
  if (journalType.value !== "all") {
    rows = rows.filter((i) => i.infraction_type === journalType.value);
  }

  // Filtre moderateur
  if (journalModerator.value !== "all") {
    rows = rows.filter((i) => i.moderator === journalModerator.value);
  }

  // Filtre date
  if (journalDateFrom.value) {
    const from = new Date(journalDateFrom.value).getTime();
    rows = rows.filter((i) => new Date(i.created_at).getTime() >= from);
  }
  if (journalDateTo.value) {
    const to = new Date(journalDateTo.value).getTime() + 86400000; // inclusif fin de journee
    rows = rows.filter((i) => new Date(i.created_at).getTime() < to);
  }

  // Tri par date desc
  rows.sort((a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime());

  return rows;
});

function resetFilters() {
  journalSearch.value = "";
  journalType.value = "all";
  journalModerator.value = "all";
  journalDateFrom.value = "";
  journalDateTo.value = "";
  hideDetections.value = true;
}

const hasActiveFilters = computed(() =>
  journalSearch.value !== "" ||
  journalType.value !== "all" ||
  journalModerator.value !== "all" ||
  journalDateFrom.value !== "" ||
  journalDateTo.value !== "" ||
  !hideDetections.value,
);

const infractionsColumns: TableColumn[] = [
  { key: "username", label: "Utilisateur" },
  { key: "infraction_type", label: "Type" },
  { key: "reason", label: "Raison" },
  { key: "moderator", label: "Moderateur" },
  { key: "created_at", label: "Date" },
  { key: "actions", label: "" },
];

async function onDeleteInfraction(id: string) {
  const ok = await confirm({ message: "Annuler cette infraction ? Cette action est irreversible." });
  if (!ok) return;
  try {
    await deleteInfraction(id);
    success("Infraction supprimee avec succes");
  } catch (e) {
    console.error("Erreur suppression infraction:", e);
    showError("Erreur lors de la suppression de l'infraction");
  }
}

async function onPurgeAll() {
  const guildId = selectedGuildId.value;
  if (!guildId) {
    showError("Selectionnez d'abord un serveur pour purger.");
    return;
  }
  const total = infractions.value?.length ?? 0;
  const ok1 = await confirm({
    message: `⚠️ SUPPRESSION DB ⚠️\n\nVous etes sur le point de supprimer DEFINITIVEMENT ${total} infraction(s) de la base de donnees pour ce serveur.\n\nCette action est IRREVERSIBLE.\n\nContinuer ?`,
  });
  if (!ok1) return;
  const ok2 = await confirm({
    message: "Derniere confirmation : toutes les infractions seront effacees en base. Vraiment proceder ?",
  });
  if (!ok2) return;
  try {
    await purgeAll(guildId);
  } catch {
    // toast deja affiche par le composable
  }
}

// --- Bans (inchange) ---
const banModalRef = ref<InstanceType<typeof BanModal> | null>(null);
const unbanError = ref<string | null>(null);
const banModalVisible = ref(false);
const banModalTarget = ref<Infraction | null>(null);

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
  } catch (e) {
    unbanError.value = String(e);
  }
}

// --- Selecteur de guild + modale "Nouvelle action" ---
const { selectedGuildId } = useGuildSelector();
const { submitting, logAction } = useModeration();
const { searchMembers } = useGuildMembers();

const actionModalVisible = ref(false);
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

function openActionModal() {
  actionError.value = null;
  actionModalVisible.value = true;
}

function closeActionModal() {
  actionModalVisible.value = false;
  actionTargetId.value = "";
  actionTargetName.value = "";
  actionReason.value = "";
  actionDuration.value = undefined;
  actionSearch.value = "";
  actionSuggestions.value = [];
  actionShowSuggestions.value = false;
  actionError.value = null;
}

function onActionSearchInput() {
  actionSuggestions.value = searchMembers(actionSearch.value);
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
  if (!actionGuildId.value || !actionTargetId.value || !actionTargetName.value || !actionReason.value) {
    actionError.value = "L'ID du serveur, la cible et la raison sont requis.";
    return;
  }
  actionError.value = null;
  try {
    const result = await logAction({
      guildId: actionGuildId.value,
      channelId: "web-panel",
      moderatorId: "web-admin",
      moderatorName: "Web Admin",
      targetId: actionTargetId.value,
      targetName: actionTargetName.value,
      actionType: actionType.value,
      reason: actionReason.value,
      gravity: actionGravity.value,
      duration: actionType.value === "mute" || actionType.value === "ban" ? actionDuration.value : undefined,
    });
    success(`${result.action_type} applique a ${result.target_name}`);
    await fetchInfractions();
    closeActionModal();
  } catch (e) {
    actionError.value = String(e);
  }
}
</script>

<template>
  <div class="moderation-hub">
    <h1>Moderation</h1>

    <!-- Tab bar (2 onglets) -->
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
    </div>

    <!-- ============================================ -->
    <!-- JOURNAL                                      -->
    <!-- ============================================ -->
    <div v-if="activeTab === 'journal'" class="tab-content">
      <!-- Filtres + bouton action -->
      <div class="journal-toolbar">
        <div class="filters-grid">
          <div class="filter-field filter-search">
            <label>Recherche</label>
            <AppInput
              v-model="journalSearch"
              placeholder="Utilisateur, ID, raison, serveur…"
            />
          </div>
          <div class="filter-field">
            <label>Type</label>
            <AppSelect v-model="journalType" :options="typeOptions" />
          </div>
          <div class="filter-field">
            <label>Moderateur</label>
            <AppSelect v-model="journalModerator" :options="moderatorOptions" />
          </div>
          <div class="filter-field">
            <label>Du</label>
            <input v-model="journalDateFrom" type="date" class="date-input" />
          </div>
          <div class="filter-field">
            <label>Au</label>
            <input v-model="journalDateTo" type="date" class="date-input" />
          </div>
        </div>

        <div class="toolbar-right">
          <label class="toggle-filter">
            <input v-model="hideDetections" type="checkbox" />
            <span>Masquer les detections AutoMod</span>
          </label>
          <button
            v-if="hasActiveFilters"
            class="reset-btn"
            @click="resetFilters"
            title="Reinitialiser les filtres"
          >
            Reinitialiser
          </button>
          <button
            class="purge-btn"
            :disabled="purging || !selectedGuildId"
            :title="selectedGuildId ? 'Supprimer toutes les infractions de la BDD' : 'Selectionnez un serveur'"
            @click="onPurgeAll"
          >
            {{ purging ? "Suppression…" : "Tout supprimer (DB)" }}
          </button>
          <AppButton variant="primary" @click="openActionModal">
            + Nouvelle action
          </AppButton>
        </div>
      </div>

      <div class="result-count">
        <strong>{{ filteredInfractions.length }}</strong>
        infraction{{ filteredInfractions.length > 1 ? "s" : "" }}
        <span v-if="filteredInfractions.length !== (infractions?.length ?? 0)" class="result-total">
          sur {{ infractions?.length ?? 0 }}
        </span>
      </div>

      <ErrorState
        v-if="infractionsError"
        :message="infractionsError"
        :retryable="true"
        @retry="fetchInfractions"
      />
      <LoadingState v-else-if="infractionsLoading" />

      <DataTable
        v-else
        :columns="infractionsColumns"
        :rows="(filteredInfractions as unknown as Record<string, unknown>[])"
        empty-message="Aucune infraction ne correspond aux filtres"
      >
        <template #cell-username="{ row }">
          <div class="user-cell">
            <span class="username">{{ (row as Record<string, unknown>).username }}</span>
            <span class="user-id">{{ (row as Record<string, unknown>).user_id }}</span>
          </div>
        </template>
        <template #cell-infraction_type="{ value }">
          <AppBadge
            :label="infractionTypeLabel(String(value))"
            :variant="isDetection(String(value)) ? 'default' : infractionTypeVariant(String(value))"
          />
        </template>
        <template #cell-created_at="{ value }">
          <span class="mono">{{ fmt(String(value)) }}</span>
        </template>
        <template #cell-actions="{ row }">
          <button
            class="cancel-btn"
            :disabled="deleting"
            title="Annuler cette infraction"
            @click.stop="onDeleteInfraction((row as Record<string, unknown>).id as string)"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M3 6h18" />
              <path d="M8 6V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
              <path d="M19 6l-1 14a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2L5 6" />
              <path d="M10 11v6" />
              <path d="M14 11v6" />
            </svg>
            <span>Annuler</span>
          </button>
        </template>
      </DataTable>
    </div>

    <!-- ============================================ -->
    <!-- BANNISSEMENTS (inchange)                     -->
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

      <p v-if="unbanError" class="ban-error">{{ unbanError }}</p>

      <LoadingState v-if="bansLoading" />

      <div v-else class="bans-columns">
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

      <BanModal
        ref="banModalRef"
        :visible="banModalVisible"
        :target="banModalTarget"
        :banning="banning"
        @close="closeBanModal"
        @confirm="onBanConfirm"
      />
    </div>

    <!-- ============================================ -->
    <!-- MODALE "Nouvelle action"                     -->
    <!-- ============================================ -->
    <div v-if="actionModalVisible" class="modal-overlay" @click.self="closeActionModal">
      <div class="modal-content action-modal">
        <div class="modal-header">
          <h3>Nouvelle action de moderation</h3>
          <button class="modal-close" @click="closeActionModal">&times;</button>
        </div>
        <div class="modal-body">
          <form class="action-form" @submit.prevent="handleActionSubmit">
            <FormField label="ID du serveur">
              <input v-model="actionGuildId" type="text" placeholder="ID du serveur" />
            </FormField>

            <FormField label="Utilisateur cible">
              <div class="autocomplete-wrapper">
                <input
                  v-model="actionSearch"
                  type="text"
                  placeholder="Rechercher un membre ou saisir un ID…"
                  @input="onActionSearchInput"
                  @focus="onActionSearchInput"
                  @blur="onActionSearchBlur"
                  autocomplete="off"
                />
                <div v-if="actionShowSuggestions" class="autocomplete-list">
                  <div
                    v-for="member in actionSuggestions"
                    :key="member.id"
                    class="autocomplete-item"
                    @mousedown="selectActionMember(member)"
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

            <div class="form-row two-col">
              <FormField label="ID cible">
                <input v-model="actionTargetId" type="text" placeholder="Auto ou manuel" />
              </FormField>
              <FormField label="Nom cible">
                <input v-model="actionTargetName" type="text" placeholder="Auto ou manuel" />
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
                <select v-model="actionGravity">
                  <option value="low">Faible</option>
                  <option value="medium">Moyen</option>
                  <option value="high">Eleve</option>
                  <option value="critical">Critique</option>
                </select>
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
              <textarea v-model="actionReason" rows="3" placeholder="Pourquoi cette action ?"></textarea>
            </FormField>

            <p v-if="actionError" class="error-msg">{{ actionError }}</p>
          </form>
        </div>
        <div class="modal-footer">
          <button class="modal-cancel" @click="closeActionModal">Annuler</button>
          <AppButton variant="primary" :disabled="submitting" @click="handleActionSubmit">
            {{ submitting ? "Application…" : `Appliquer ${actionType}` }}
          </AppButton>
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

/* ---- Journal toolbar + filters ---- */
.journal-toolbar {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 16px;
  padding: 16px;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
}

.filters-grid {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr 1fr;
  gap: 12px;
}

@media (max-width: 1200px) {
  .filters-grid {
    grid-template-columns: 1fr 1fr 1fr;
  }
  .filter-search { grid-column: 1 / -1; }
}

@media (max-width: 700px) {
  .filters-grid { grid-template-columns: 1fr; }
}

.filter-field {
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 0;
}

.filter-field label {
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  color: var(--text-secondary);
  letter-spacing: 0.3px;
}

.date-input {
  background-color: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 10px;
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
  color-scheme: dark;
}

.date-input:focus {
  border-color: var(--accent);
}

.toolbar-right {
  display: flex;
  align-items: center;
  justify-content: flex-end;
  gap: 12px;
  flex-wrap: wrap;
}

.toggle-filter {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  user-select: none;
}

.toggle-filter input[type="checkbox"] {
  accent-color: var(--accent);
  cursor: pointer;
}

.toggle-filter:hover {
  color: var(--text-primary);
}

.reset-btn {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 7px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}

.reset-btn:hover {
  color: var(--text-primary);
  border-color: var(--accent);
}

.purge-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 7px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
}

.purge-btn:hover:not(:disabled) {
  background: var(--danger);
  color: white;
}

.purge-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.result-count {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 12px;
  padding: 0 4px;
}

.result-count strong {
  color: var(--text-primary);
  font-weight: 700;
  font-size: 13px;
}

.result-total {
  opacity: 0.7;
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

.cancel-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background-color: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 8px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.cancel-btn:hover:not(:disabled) {
  background-color: var(--danger);
  color: white;
}

.cancel-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.cancel-btn svg {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
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

@media (max-width: 900px) {
  .bans-columns { grid-template-columns: 1fr; }
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

/* ---- Modale "Nouvelle action" ---- */
.modal-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
}

.modal-content {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  width: 100%;
  max-width: 560px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 20px 60px rgba(0, 0, 0, 0.4);
}

.action-modal {
  overflow: hidden;
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
  font-size: 26px;
  cursor: pointer;
  line-height: 1;
}

.modal-close:hover {
  color: var(--text-primary);
}

.modal-body {
  padding: 20px;
  overflow-y: auto;
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
:deep(.form-field) textarea:focus {
  border-color: var(--accent);
}

:deep(.form-field) textarea { resize: vertical; }

.error-msg { color: var(--danger); font-size: 13px; }

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
