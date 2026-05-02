<script setup lang="ts">
import { ref, computed, watch, onMounted, onBeforeUnmount } from "vue";
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
import StrikesPage from "./StrikesPage.vue";
import NotesPage from "./NotesPage.vue";
import EvidencePage from "./EvidencePage.vue";
import ReviewPage from "./ReviewPage.vue";
import RemindersPage from "./RemindersPage.vue";
import CancelConductBanModal from "../molecules/CancelConductBanModal.vue";
import { conductService } from "@/services/conductService";
import ErrorState from "../atoms/ErrorState.vue";
import EmptyState from "../atoms/EmptyState.vue";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

const { visible: rbacVisible } = useComponentVisibility();
import AppButton from "../atoms/AppButton.vue";
import FormField from "../atoms/FormField.vue";
import { infractionTypeVariant } from "../../utils/variants";
import { safeImageUrl } from "../../utils/safeUrl";
import { useFormatDate } from "../../composables/useFormatDate";
import { useBans } from "../../composables/useBans";
import { useConfirm } from "../../composables/useConfirm";
import { useModeration } from "../../composables/useModeration";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useGuildMembers } from "../../composables/useGuildMembers";
import { useSharedUserLookup } from "../../composables/useSharedUserLookup";

const { success, error: showError } = useToast();
const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();

// --- Tabs ---
const activeTab = ref<"journal" | "bans" | "tracking" | "workflow">("journal");
const bulkMenuOpen = ref(false);
const trackingSubTab = ref<"strikes" | "notes-evidence">("strikes");
const { sharedUserId } = useSharedUserLookup();

/** Bouton de la ligne du Journal -> bascule vers Notes & Preuves avec l'ID user pre-rempli. */
function openNotesEvidence(userId: string) {
  if (!userId) return;
  sharedUserId.value = userId;
  activeTab.value = "tracking";
  trackingSubTab.value = "notes-evidence";
}
const workflowSubTab = ref<"review" | "reminders">("review");
function closeBulkMenu() { bulkMenuOpen.value = false; }
onMounted(() => document.addEventListener("click", closeBulkMenu));
onBeforeUnmount(() => document.removeEventListener("click", closeBulkMenu));

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
const journalStatus = ref<"all" | "detection" | "action">("all");
const journalDateFrom = ref<string>("");
const journalDateTo = ref<string>("");
// Les detections AutoMod sans sanction ("none" / "") polluent le journal :
// masquees par defaut, decochable pour tout voir.
const hideDetections = ref(true);

const statusOptions = [
  { value: "all", label: "Tous les statuts" },
  { value: "detection", label: "Propositions" },
  { value: "action", label: "Appliquees" },
];

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

  // Masque les bans appliques (permanent ou temp) : ils sont deja visibles
  // dans l'onglet "Bannis actifs". On garde uniquement les propositions de
  // ban (source=detection), car le mod doit pouvoir les appliquer ici.
  rows = rows.filter((i) => {
    const isBan = i.infraction_type === "ban_permanent"
      || i.infraction_type === "ban_temp"
      || i.infraction_type === "ban";
    const isApplied = (i.source ?? "detection") === "action";
    return !(isBan && isApplied);
  });

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

  // Filtre statut (proposition / appliquee)
  if (journalStatus.value !== "all") {
    rows = rows.filter((i) => (i.source ?? "detection") === journalStatus.value);
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
  journalStatus.value = "all";
  journalDateFrom.value = "";
  journalDateTo.value = "";
  hideDetections.value = true;
}

const hasActiveFilters = computed(() =>
  journalSearch.value !== "" ||
  journalType.value !== "all" ||
  journalModerator.value !== "all" ||
  journalStatus.value !== "all" ||
  journalDateFrom.value !== "" ||
  journalDateTo.value !== "" ||
  !hideDetections.value,
);

const infractionsColumns: TableColumn[] = [
  { key: "username", label: "Utilisateur" },
  { key: "infraction_type", label: "Type" },
  { key: "source", label: "Choix" },
  { key: "reason", label: "Raison" },
  { key: "moderator", label: "Moderateur" },
  { key: "created_at", label: "Date" },
  { key: "actions", label: "" },
];

// Modale specifique pour annuler une proposition de ban "Points de conduite".
// Permet de redonner des points avant suppression, sinon le worker
// recreera la proposition. Cf sync_ban_proposals.
const conductBanModalVisible = ref(false);
const conductBanModalTarget = ref<Infraction | null>(null);

function isConductProposal(row: Record<string, unknown>): boolean {
  const source = row.source as string | undefined;
  const actionType = String(row.infraction_type ?? "").toLowerCase();
  const reason = String(row.reason ?? "");
  return (
    source === "detection" &&
    actionType === "ban" &&
    reason.startsWith("Points de conduite")
  );
}

async function onDeleteInfraction(row: Record<string, unknown>) {
  const id = row.id as string;
  const source = (row.source as "detection" | "action" | undefined) ?? "detection";
  const actionType = String(row.infraction_type ?? "").toLowerCase();
  const isBan = source === "action" && actionType.startsWith("ban");
  const isMute = source === "action" && (actionType.startsWith("mute") || actionType === "timeout");

  // Cas special : proposition de ban liee aux points de conduite. On ouvre
  // une modale dediee qui permet de redonner des points avant suppression.
  if (isConductProposal(row)) {
    conductBanModalTarget.value = row as unknown as Infraction;
    conductBanModalVisible.value = true;
    return;
  }

  let message: string;
  if (isBan) {
    message = "Annuler ce BAN ? L'utilisateur sera debanni du serveur Discord et la ligne supprimee de la BDD. Cette action est irreversible.";
  } else if (isMute) {
    message = "Annuler ce MUTE ? Le timeout sera retire sur Discord et la ligne supprimee de la BDD. Cette action est irreversible.";
  } else if (source === "action") {
    message = "Annuler cette action appliquee ? La ligne sera supprimee de la BDD. Cette action est irreversible.";
  } else {
    message = "Annuler cette detection ? Elle sera supprimee de la BDD. Cette action est irreversible.";
  }

  const ok = await confirm({ message });
  if (!ok) return;
  try {
    await deleteInfraction(id, source);
  } catch (e) {
    console.error("Erreur suppression infraction:", e);
    showError("Erreur lors de la suppression");
  }
}

function closeConductBanModal() {
  conductBanModalVisible.value = false;
  conductBanModalTarget.value = null;
}

async function onConductBanModalConfirm(grant: number | null) {
  const target = conductBanModalTarget.value;
  if (!target) return;
  try {
    if (grant !== null && grant > 0) {
      await conductService.adjustPoints(
        target.server,
        target.user_id,
        grant,
        "Annulation manuelle de la proposition de ban",
      );
    }
    await deleteInfraction(target.id, "detection");
    closeConductBanModal();
  } catch (e) {
    console.error("Erreur annulation proposition conduite:", e);
    showError("Erreur lors de l'annulation");
    closeConductBanModal();
  }
}

// --- Appliquer une detection (transformer en action effective) ---
const applying = ref(false);

async function onApplyDetection(row: Record<string, unknown>) {
  const id = row.id as string;
  const actionType = String(row.infraction_type ?? "").toLowerCase();
  const guildId = String(row.server ?? "");
  const userId = String(row.user_id ?? "");
  const username = String(row.username ?? userId);
  const reason = String(row.reason ?? "Applique depuis le panneau admin");

  if (!guildId || !userId) {
    showError("Guild ou user manquant sur cette detection");
    return;
  }

  const isBan = actionType === "ban";
  const isMute = actionType === "mute" || actionType === "timeout";
  const isWarn = actionType === "warn";
  const duration = typeof row.duration === "number" ? (row.duration as number) : undefined;

  const label = isBan ? "BAN" : isMute ? "MUTE" : isWarn ? "AVERTISSEMENT" : actionType.toUpperCase();
  const detail = isBan
    ? "L'utilisateur sera effectivement banni du serveur Discord."
    : isMute
      ? `Un timeout Discord sera applique (${duration ?? 3600}s) et l'action sera loguee en DB.`
      : "Un avertissement sera enregistre en DB.";

  const ok = await confirm({
    message: `Appliquer ${label} a ${username} ?\n\n${detail}\n\nRaison : ${reason}`,
  });
  if (!ok) return;

  applying.value = true;
  try {
    const { moderationService } = await import("@/services/moderationService");
    if (isBan) {
      await moderationService.executeBan(guildId, userId, reason);
    } else if (isMute) {
      await moderationService.executeMute(guildId, userId, reason, duration, username);
    } else {
      // warn / autre : juste logger l'action en DB (pas d'effet Discord).
      await logAction({
        guildId,
        channelId: "web-panel",
        moderatorId: "web-admin",
        moderatorName: "Web Admin",
        targetId: userId,
        targetName: username,
        actionType,
        reason,
        gravity: "medium",
      });
    }
    // Supprime la detection d'origine pour eviter le doublon visuel.
    await deleteInfraction(id, "detection");
    success(`${label} applique a ${username}`);
  } catch (e) {
    console.error("Erreur apply detection:", e);
    showError("Erreur lors de l'application de la detection");
  } finally {
    applying.value = false;
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
    message:
      `⚠️ Vider le journal (DB seule) ⚠️\n\n` +
      `Cette action supprime ${total} infraction(s) de la base de données POUR CE SERVEUR.\n\n` +
      `IMPORTANT : ça ne touche PAS Discord :\n` +
      `  • les bannissements actifs RESTENT actifs\n` +
      `  • les mutes / timeouts en cours RESTENT actifs\n` +
      `  • aucun DM de grâce n'est envoyé\n\n` +
      `Pour vraiment annuler une sanction (avec unban Discord), utilise le bouton Annuler ligne par ligne.\n\n` +
      `Cette suppression est IRRÉVERSIBLE. Continuer ?`,
  });
  if (!ok1) return;
  const ok2 = await confirm({
    message: "Dernière confirmation : vider le journal pour ce serveur ? (les sanctions Discord ne seront PAS levées)",
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

// --- Bulk unban : debannir tous les bannis actifs visibles ---
const bulkUnbanning = ref(false);
const unbanProgress = ref(0);
const unbanTotal = ref(0);

async function onUnbanAll() {
  unbanError.value = null;
  const targets = [...filteredConfirmed.value];
  if (targets.length === 0) return;

  // Double confirmation : opération sensible et bruyante (peut générer du
  // rate-limit Discord si la liste est grande).
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
    // Petite pause pour ne pas saturer Discord rate-limit (50 req/s
    // global, mais unban_user a son propre bucket).
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

// --- Selecteur de guild + modale "Nouvelle action" ---
const { selectedGuildId } = useGuildSelector();
const { submitting, logAction } = useModeration();
const { members, searchMembers } = useGuildMembers();

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

/// Detecte si la valeur saisie ressemble a un snowflake Discord (17-20 chiffres).
function isSnowflake(s: string): boolean {
  return /^\d{17,20}$/.test(s.trim());
}

function onActionSearchInput() {
  const q = actionSearch.value.trim();

  // Cas 1 : l'user a colle un ID Discord brut (snowflake numerique).
  // On remplit actionTargetId direct et on tente un lookup local pour
  // recuperer le username (sinon on met l'ID en fallback name).
  if (isSnowflake(q)) {
    actionTargetId.value = q;
    const member = members.value?.find((m) => m.id === q);
    if (member) {
      actionTargetName.value = member.display_name || member.username;
    } else if (!actionTargetName.value) {
      actionTargetName.value = q; // fallback : l'ID sera affiche comme name
    }
    actionSuggestions.value = [];
    actionShowSuggestions.value = false;
    return;
  }

  // Cas 2 : recherche par nom -> autocomplete normal.
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
  // Validation : on exige guild_id + raison + au moins un ID OU un nom.
  // Si l'user a saisi juste un nom (autocomplete sans clic), on essaie
  // un dernier match exact sur la liste des membres avant d'echouer.
  if (!actionGuildId.value || !actionReason.value) {
    actionError.value = "L'ID du serveur et la raison sont requis.";
    return;
  }

  // Resolution finale : si pas d'ID mais un nom, tenter un match exact.
  if (!actionTargetId.value.trim() && actionTargetName.value.trim()) {
    const name = actionTargetName.value.trim().toLowerCase();
    const match = members.value?.find(
      (m) =>
        m.username.toLowerCase() === name ||
        (m.display_name?.toLowerCase() ?? "") === name,
    );
    if (match) {
      actionTargetId.value = match.id;
    }
  }
  // Idem inverse : juste l'ID, pas de nom -> fallback nom = ID.
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
        Bannis actifs
      </button>
      <button
        :class="['hub-tab', { active: activeTab === 'tracking' }]"
        @click="activeTab = 'tracking'"
      >
        Suivi utilisateur
      </button>
      <button
        :class="['hub-tab', { active: activeTab === 'workflow' }]"
        @click="activeTab = 'workflow'"
      >
        Workflow
      </button>
    </div>

    <!-- ============================================ -->
    <!-- JOURNAL                                      -->
    <!-- ============================================ -->
    <div v-if="activeTab === 'journal'" class="tab-content">
      <!-- Filtres + bouton action -->
      <div class="card journal-toolbar">
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
            <label>Statut</label>
            <AppSelect v-model="journalStatus" :options="statusOptions" />
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
            <span>Masquer les détections AutoMod</span>
            <span class="switch">
              <input v-model="hideDetections" type="checkbox" />
              <span class="slider" aria-hidden="true"></span>
            </span>
          </label>
          <button
            v-if="hasActiveFilters"
            class="reset-btn"
            @click="resetFilters"
            title="Reinitialiser les filtres"
          >
            Reinitialiser
          </button>
          <div v-if="rbacVisible('db.purge.audit_logs')" class="bulk-menu-wrap" @click.stop>
            <button
              class="bulk-menu-btn"
              :disabled="!selectedGuildId"
              :title="selectedGuildId ? 'Actions de masse (owner uniquement)' : 'Selectionnez un serveur'"
              @click="bulkMenuOpen = !bulkMenuOpen"
            >
              ⋯ Actions de masse ▾
            </button>
            <div v-if="bulkMenuOpen" class="bulk-menu">
              <button
                class="bulk-item danger"
                :disabled="purging"
                title="Vide le journal de la base de données. NE débannit PAS et NE retire PAS les mutes sur Discord."
                @click="bulkMenuOpen = false; onPurgeAll()"
              >
                🗑 {{ purging ? "Suppression…" : "Vider le journal (DB seule)" }}
              </button>
            </div>
          </div>
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
            <strong v-if="(row as Record<string, unknown>).display_name" class="display-name">
              {{ (row as Record<string, unknown>).display_name }}
            </strong>
            <span class="username">@{{ (row as Record<string, unknown>).username }}</span>
            <span class="user-id">{{ (row as Record<string, unknown>).user_id }}</span>
          </div>
        </template>
        <template #cell-infraction_type="{ value }">
          <AppBadge
            :label="infractionTypeLabel(String(value))"
            :variant="isDetection(String(value)) ? 'default' : infractionTypeVariant(String(value))"
          />
        </template>
        <template #cell-source="{ row, value }">
          <span
            v-if="value === 'detection' && !isDetection(String((row as Record<string, unknown>).infraction_type))"
            class="source-chip proposal"
            title="Detection AutoMod : proposition, pas encore appliquee"
          >
            Proposition
          </span>
          <span
            v-else-if="value === 'action'"
            class="source-chip applied"
            title="Sanction effectivement appliquee par un moderateur ou un bot"
          >
            Applique
          </span>
          <span v-else class="source-chip neutral">—</span>
        </template>
        <template #cell-created_at="{ value }">
          <span class="mono">{{ fmt(String(value)) }}</span>
        </template>
        <template #cell-actions="{ row }">
          <div class="action-buttons">
            <button
              class="notes-btn"
              title="Voir / ajouter notes et preuves pour cet utilisateur"
              @click.stop="openNotesEvidence(String((row as Record<string, unknown>).user_id))"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
                <polyline points="14 2 14 8 20 8" />
              </svg>
              <span>📎</span>
            </button>
            <button
              v-if="(row as Record<string, unknown>).source === 'detection'
                    && !isDetection(String((row as Record<string, unknown>).infraction_type))"
              class="apply-btn"
              :disabled="applying"
              title="Appliquer cette proposition (ban/mute/warn)"
              @click.stop="onApplyDetection(row as Record<string, unknown>)"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="20 6 9 17 4 12" />
              </svg>
              <span>Appliquer</span>
            </button>
            <button
              class="cancel-btn"
              :disabled="deleting"
              title="Annuler cette entree (si ban applique, unban Discord inclus)"
              @click.stop="onDeleteInfraction(row as Record<string, unknown>)"
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
          </div>
        </template>
      </DataTable>
    </div>

    <!-- ============================================ -->
    <!-- BANNIS ACTIFS — etat courant Discord uniquement                -->
    <!-- ============================================ -->
    <div v-if="activeTab === 'bans'" class="tab-content">
      <div class="bans-toolbar">
        <input
          v-model="bansSearchQuery"
          type="text"
          placeholder="Rechercher par nom, ID ou raison..."
          class="search-input"
        />
        <button
          v-if="rbacVisible('moderation.bulk_unban')"
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
              <div class="user-avatar-placeholder confirmed-avatar">{{ (ban.target_display_name || ban.target_name).charAt(0).toUpperCase() }}</div>
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
    <!-- SUIVI UTILISATEUR — strikes / notes / preuves -->
    <!-- ============================================ -->
    <div v-if="activeTab === 'tracking'" class="tab-content">
      <div class="sub-tabs">
        <button
          :class="['sub-tab', { active: trackingSubTab === 'strikes' }]"
          @click="trackingSubTab = 'strikes'"
        >Avertissements</button>
        <button
          :class="['sub-tab', { active: trackingSubTab === 'notes-evidence' }]"
          @click="trackingSubTab = 'notes-evidence'"
        >Notes &amp; Preuves</button>
      </div>
      <StrikesPage v-if="trackingSubTab === 'strikes'" />
      <div v-else-if="trackingSubTab === 'notes-evidence'">
        <!-- Champ ID user partagé entre Notes et Preuves -->
        <section class="card shared-user-bar">
          <h2>👤 Utilisateur ciblé</h2>
          <p class="muted small">
            Saisis un ID Discord — les notes et les preuves de modération
            de cet utilisateur s'affichent dans les deux panneaux ci-dessous.
          </p>
          <div class="lookup">
            <input
              v-model="sharedUserId"
              placeholder="ID Discord de l'utilisateur (ex: 123456789012345678)"
              type="text"
            />
            <button v-if="sharedUserId" class="btn-secondary" @click="sharedUserId = ''">
              Effacer
            </button>
          </div>
        </section>

        <div v-if="sharedUserId.trim()" class="stacked-sections">
          <section class="stacked-block">
            <h2 class="stacked-title">📝 Notes de modération</h2>
            <p class="stacked-hint">Notes libres attachées à cet utilisateur (visibles entre modérateurs).</p>
            <NotesPage embedded />
          </section>
          <section class="stacked-block">
            <h2 class="stacked-title">📎 Preuves</h2>
            <p class="stacked-hint">Pièces jointes (URL/description) attachées à une action de modération précise.</p>
            <EvidencePage embedded />
          </section>
        </div>
        <div v-else class="empty-shared">
          Saisis un ID utilisateur ci-dessus pour voir ses notes et preuves.
        </div>
      </div>
    </div>

    <!-- ============================================ -->
    <!-- WORKFLOW — file de revue + rappels           -->
    <!-- ============================================ -->
    <div v-if="activeTab === 'workflow'" class="tab-content">
      <div class="sub-tabs">
        <button
          :class="['sub-tab', { active: workflowSubTab === 'review' }]"
          @click="workflowSubTab = 'review'"
        >Revue AutoMod</button>
        <button
          :class="['sub-tab', { active: workflowSubTab === 'reminders' }]"
          @click="workflowSubTab = 'reminders'"
        >Rappels</button>
      </div>
      <ReviewPage v-if="workflowSubTab === 'review'" />
      <RemindersPage v-else-if="workflowSubTab === 'reminders'" />
    </div>

    <!-- Modale d annulation des propositions "Points de conduite" :
         monte au top-level pour rester accessible depuis l onglet journal. -->
    <CancelConductBanModal
      :visible="conductBanModalVisible"
      :target="conductBanModalTarget"
      @close="closeConductBanModal"
      @confirm="onConductBanModalConfirm"
    />

    <!-- ============================================ -->
    <!-- MODALE "Nouvelle action"                     -->
    <!-- ============================================ -->
    <div v-if="actionModalVisible" class="modal-overlay" @click.self="closeActionModal">
      <div class="card modal-content action-modal">
        <div class="modal-header">
          <h3>Nouvelle action de moderation</h3>
          <button class="modal-close" @click="closeActionModal">&times;</button>
        </div>
        <div class="modal-body">
          <form class="action-form" @submit.prevent="handleActionSubmit">
            <FormField label="ID du serveur">
              <input v-model="actionGuildId" type="text" placeholder="ID du serveur" />
            </FormField>

            <FormField label="Utilisateur cible (nom OU ID Discord)">
              <div class="autocomplete-wrapper">
                <input
                  v-model="actionSearch"
                  type="text"
                  placeholder="Tapez un pseudo, ou collez un ID Discord (17-20 chiffres)…"
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
  margin-bottom: 18px;
  font-size: 1.6rem;
  font-weight: 700;
  /* Gradient shimmer cohérent avec /stats et /modstats. */
  background: linear-gradient(
    90deg,
    var(--text-primary) 0%,
    color-mix(in srgb, var(--accent) 60%, var(--text-primary)) 50%,
    var(--text-primary) 100%
  );
  background-size: 200% auto;
  -webkit-background-clip: text;
  background-clip: text;
  -webkit-text-fill-color: transparent;
  color: transparent;
  animation: mod-title-shimmer 10s linear infinite;
  letter-spacing: 0.3px;
}
@keyframes mod-title-shimmer {
  0%   { background-position: 200% center; }
  100% { background-position: -200% center; }
}

/* ---- Tab bar polished ---- */
.hub-tabs {
  display: flex;
  gap: 4px;
  position: relative;
  margin-bottom: 24px;
  padding: 4px;
  background-color: color-mix(in srgb, var(--bg-card) 80%, transparent);
  backdrop-filter: blur(6px);
  -webkit-backdrop-filter: blur(6px);
  border: 1px solid var(--border);
  border-radius: 12px;
  width: fit-content;
  box-shadow:
    inset 0 1px 2px rgba(0, 0, 0, 0.18),
    0 1px 0 color-mix(in srgb, white 6%, transparent);
}

.hub-tab {
  position: relative;
  padding: 8px 22px;
  background: none;
  border: none;
  border-radius: 8px;
  color: var(--text-secondary);
  font-weight: 600;
  font-size: 0.9rem;
  cursor: pointer;
  transition: color 0.2s ease,
    background 0.25s ease,
    box-shadow 0.25s ease;
}

.hub-tab::after {
  content: "";
  position: absolute;
  left: 50%;
  bottom: 4px;
  width: 0;
  height: 2px;
  border-radius: 2px;
  background: var(--accent);
  transform: translateX(-50%);
  transition: width 0.25s cubic-bezier(0.4, 0, 0.2, 1);
}

.hub-tab:hover:not(.active) {
  color: var(--text-primary);
  background-color: color-mix(in srgb, var(--accent) 8%, transparent);
}
.hub-tab:hover:not(.active)::after {
  width: 50%;
}

.hub-tab.active {
  color: white;
  background: linear-gradient(135deg,
    var(--accent),
    color-mix(in srgb, var(--accent) 75%, var(--accent-alt, #a855f7)));
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 35%, transparent),
    inset 0 -1px 0 color-mix(in srgb, black 15%, transparent),
    0 2px 10px color-mix(in srgb, var(--accent) 35%, transparent);
  text-shadow: 0 1px 1px rgba(0, 0, 0, 0.12);
}

.hub-tab:active {
  transform: scale(0.96);
  transition-duration: 0.08s;
}

.tab-content {
  animation: fadeSlideIn 0.3s ease-out;
}

@keyframes fadeSlideIn {
  from { opacity: 0; transform: translateY(6px); }
  to   { opacity: 1; transform: translateY(0); }
}

/* ---- Journal toolbar + filters ---- */
.journal-toolbar {
  display: flex;
  flex-direction: column;
  gap: var(--space-md);
  margin-bottom: var(--space-lg);
}

.filters-grid {
  display: grid;
  grid-template-columns: 2fr 1fr 1fr 1fr 1fr 1fr;
  gap: 12px;
}

@media (max-width: 1400px) {
  .filters-grid {
    grid-template-columns: repeat(3, 1fr);
  }
  .filter-search { grid-column: 1 / -1; }
}

@media (max-width: 800px) {
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
  gap: 10px;
  font-size: 12px;
  color: var(--text-secondary);
  cursor: pointer;
  user-select: none;
  transition: color 0.2s ease;
}
.toggle-filter:hover {
  color: var(--text-primary);
}

/* ── Switch (slider) — remplace l'ancienne checkbox ──── */
.switch {
  position: relative;
  display: inline-block;
  width: 36px;
  height: 20px;
  flex-shrink: 0;
}
.switch input {
  position: absolute;
  width: 100%;
  height: 100%;
  margin: 0;
  opacity: 0;
  cursor: pointer;
  z-index: 2;
}
.switch .slider {
  position: absolute;
  inset: 0;
  background: color-mix(in srgb, var(--bg-card) 80%, transparent);
  border: 1px solid var(--border);
  border-radius: 999px;
  transition: background-color 0.25s ease, border-color 0.25s ease;
  box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.18);
}
.switch .slider::before {
  content: "";
  position: absolute;
  top: 2px;
  left: 2px;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: linear-gradient(180deg, white, #d8d8e0);
  box-shadow: 0 1px 3px rgba(0, 0, 0, 0.35);
  transition: transform 0.28s cubic-bezier(0.34, 1.56, 0.64, 1),
    background 0.25s ease;
}
.switch input:checked + .slider {
  background: linear-gradient(135deg,
    var(--accent),
    color-mix(in srgb, var(--accent) 75%, var(--accent-alt, #a855f7)));
  border-color: color-mix(in srgb, var(--accent) 70%, var(--border));
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 30%, transparent),
    0 0 8px color-mix(in srgb, var(--accent) 35%, transparent);
}
.switch input:checked + .slider::before {
  transform: translateX(16px);
}
.switch input:focus-visible + .slider {
  outline: 2px solid var(--accent);
  outline-offset: 2px;
}
.switch input:active + .slider::before {
  width: 18px;
}

@media (prefers-reduced-motion: reduce) {
  .switch .slider,
  .switch .slider::before { transition: none; }
}

.reset-btn {
  background:
    linear-gradient(180deg,
      color-mix(in srgb, white 4%, var(--bg-card)),
      var(--bg-card));
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 7px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease,
    background 0.25s ease,
    border-color 0.2s ease,
    box-shadow 0.25s ease;
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 6%, transparent);
}

.reset-btn:hover {
  color: var(--accent);
  border-color: color-mix(in srgb, var(--accent) 55%, var(--border));
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--accent) 10%, var(--bg-card)),
    color-mix(in srgb, var(--accent) 6%, var(--bg-card)));
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 10%, transparent),
    0 4px 12px color-mix(in srgb, var(--accent) 18%, transparent);
}
.reset-btn:active {
  transform: scale(0.97);
  transition-duration: 0.08s;
}

.purge-btn {
  background:
    linear-gradient(180deg,
      color-mix(in srgb, var(--danger) 6%, var(--bg-card)),
      var(--bg-card));
  color: var(--danger);
  border: 1px solid color-mix(in srgb, var(--danger) 60%, var(--border));
  border-radius: 8px;
  padding: 7px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease,
    background 0.25s ease,
    border-color 0.2s ease,
    box-shadow 0.25s ease;
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 6%, transparent);
}

.purge-btn:hover:not(:disabled) {
  color: white;
  border-color: var(--danger);
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--danger) 90%, white),
    var(--danger));
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 25%, transparent),
    0 4px 14px color-mix(in srgb, var(--danger) 35%, transparent);
}
.purge-btn:active:not(:disabled) {
  transform: scale(0.97);
  transition-duration: 0.08s;
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

.source-chip {
  display: inline-block;
  font-size: 11px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.3px;
  padding: 3px 10px;
  border-radius: 10px;
  border: 1px solid;
  white-space: nowrap;
}

.source-chip.proposal {
  color: #fee75c;
  border-color: rgba(254, 231, 92, 0.5);
  background-color: rgba(254, 231, 92, 0.08);
}

.source-chip.applied {
  color: #57f287;
  border-color: rgba(87, 242, 135, 0.5);
  background-color: rgba(87, 242, 135, 0.08);
}

.source-chip.neutral {
  color: var(--text-secondary);
  border-color: var(--border);
  background-color: transparent;
  font-weight: 400;
}

.mono {
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  font-size: 12px;
}

.action-buttons {
  display: inline-flex;
  align-items: center;
  gap: 8px;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.notes-btn {
  display: inline-flex;
  align-items: center;
  gap: 4px;
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 6px 10px;
  font-size: 12px;
  cursor: pointer;
  transition: color var(--transition-fast), border-color var(--transition-fast), background-color var(--transition-fast);
}
.notes-btn:hover {
  color: var(--accent);
  border-color: var(--accent);
  background-color: var(--bg-hover);
}
.notes-btn svg { width: 14px; height: 14px; }

.apply-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background:
    linear-gradient(180deg,
      color-mix(in srgb, #57f287 8%, var(--bg-card)),
      var(--bg-card));
  color: #57f287;
  border: 1px solid color-mix(in srgb, #57f287 60%, var(--border));
  border-radius: 8px;
  padding: 8px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease,
    background 0.25s ease,
    border-color 0.2s ease,
    box-shadow 0.25s ease;
  white-space: nowrap;
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 6%, transparent);
}

.apply-btn:hover:not(:disabled) {
  color: #0a0a0a;
  border-color: #57f287;
  background: linear-gradient(180deg,
    color-mix(in srgb, #57f287 95%, white),
    #57f287);
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 35%, transparent),
    0 4px 14px color-mix(in srgb, #57f287 40%, transparent);
}
.apply-btn:active:not(:disabled) {
  transform: scale(0.97);
  transition-duration: 0.08s;
}

.apply-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.apply-btn svg {
  width: 14px;
  height: 14px;
  flex-shrink: 0;
}

.cancel-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background:
    linear-gradient(180deg,
      color-mix(in srgb, var(--danger) 6%, var(--bg-card)),
      var(--bg-card));
  color: var(--danger);
  border: 1px solid color-mix(in srgb, var(--danger) 60%, var(--border));
  border-radius: 8px;
  padding: 8px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease,
    background 0.25s ease,
    border-color 0.2s ease,
    box-shadow 0.25s ease;
  white-space: nowrap;
  box-shadow: inset 0 1px 0 color-mix(in srgb, white 6%, transparent);
}

.cancel-btn:hover:not(:disabled) {
  color: white;
  border-color: var(--danger);
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--danger) 90%, white),
    var(--danger));
  box-shadow:
    inset 0 1px 0 color-mix(in srgb, white 25%, transparent),
    0 4px 14px color-mix(in srgb, var(--danger) 40%, transparent);
}
.cancel-btn:active:not(:disabled) {
  transform: scale(0.97);
  transition-duration: 0.08s;
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
  transition: border-color var(--transition-base);
}

.search-input:focus {
  border-color: var(--accent);
}

.search-input::placeholder {
  color: var(--text-secondary);
  opacity: 0.6;
}

.bans-list-single {
  width: 100%;
}
.bans-list-single .ban-list {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
  gap: 16px;
}

.bans-toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}
.bans-toolbar .search-input {
  flex: 1;
  min-width: 240px;
}
.sub-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--border);
}
.sub-tab {
  background: transparent;
  border: 0;
  padding: 8px 14px;
  font-size: 13px;
  color: var(--text-secondary);
  cursor: pointer;
  border-bottom: 2px solid transparent;
  font-weight: 600;
}
.sub-tab:hover { color: var(--text-primary); }
.sub-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.shared-user-bar { margin-bottom: 20px; }
.shared-user-bar .lookup { display: flex; gap: 10px; margin-top: 12px; }
.shared-user-bar .lookup input {
  flex: 1;
  padding: 10px 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 14px;
  font-family: "JetBrains Mono", monospace;
}
.shared-user-bar .lookup input:focus { outline: 1px solid var(--accent); border-color: var(--accent); }
.empty-shared {
  padding: 60px 20px;
  text-align: center;
  color: var(--text-secondary);
  font-style: italic;
  border: 1px dashed var(--border);
  border-radius: 10px;
}

.stacked-sections {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 24px;
  align-items: start;
}
@media (max-width: 1100px) {
  .stacked-sections { grid-template-columns: 1fr; }
}
.stacked-block {
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 20px;
  background: var(--bg-secondary);
}
.stacked-title {
  font-size: 16px;
  font-weight: 700;
  margin: 0 0 6px 0;
  color: var(--text-primary);
}
.stacked-hint {
  font-size: 12px;
  color: var(--text-secondary);
  margin: 0 0 14px 0;
}

.bulk-menu-wrap { position: relative; display: inline-block; }
.bulk-menu-btn {
  background: transparent;
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 14px;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  white-space: nowrap;
  transition: color var(--transition-fast), background-color var(--transition-fast), border-color var(--transition-fast);
}
.bulk-menu-btn:hover:not(:disabled) {
  color: var(--accent);
  border-color: var(--accent);
  background-color: var(--bg-hover);
}
.bulk-menu-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.bulk-menu {
  position: absolute;
  right: 0;
  top: calc(100% + 6px);
  min-width: 240px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  box-shadow: 0 8px 24px rgba(0,0,0,0.35);
  padding: 4px;
  z-index: 50;
}
.bulk-item {
  display: block;
  width: 100%;
  text-align: left;
  background: transparent;
  border: 0;
  border-radius: 6px;
  padding: 9px 12px;
  font-size: 13px;
  color: var(--text-primary);
  cursor: pointer;
}
.bulk-item:hover:not(:disabled) { background: var(--bg-hover); }
.bulk-item.danger { color: var(--danger); }
.bulk-item.danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger) 10%, transparent); }
.bulk-item:disabled { opacity: 0.5; cursor: not-allowed; }

.unban-all-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid color-mix(in srgb, var(--danger) 50%, var(--border));
  border-radius: 8px;
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
.unban-all-btn:active:not(:disabled) {
  transform: scale(0.97);
}
.unban-all-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
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

/* .ban-card herite de .card (bg/border/radius/padding). Seules les variantes
   confirmed/proposal ajoutent une bordure gauche coloree pour distinguer. */
.ban-card {
  /* Stagger entrance + hover lift cosy. */
  opacity: 0;
  animation: ban-card-enter 0.4s ease-out forwards;
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1),
    border-color 0.25s ease,
    box-shadow 0.3s ease;
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

.ban-card.confirmed {
  border-left: 3px solid var(--danger);
}
.ban-card.confirmed:hover {
  transform: translateY(-2px);
  border-left-color: var(--danger);
  box-shadow: 0 8px 22px color-mix(in srgb, var(--danger) 14%, transparent);
}

.ban-card.proposal {
  border-left: 3px solid var(--warning);
}
.ban-card.proposal:hover {
  transform: translateY(-2px);
  box-shadow: 0 8px 22px color-mix(in srgb, var(--warning, #f59e0b) 14%, transparent);
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

.display-name {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.display-name + .username {
  font-weight: 400;
  font-size: 12px;
  color: var(--text-secondary);
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
  transition: opacity var(--transition-base);
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
  transition: all var(--transition-base);
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
  background: var(--modal-overlay);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 20px;
}

.modal-content {
  padding: 0; /* override .card : header/body/footer gerent leur padding */
  width: 100%;
  max-width: 560px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-xl);
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
  transition: background var(--transition-base);
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
  font-size: 12px;
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

@media (prefers-reduced-motion: reduce) {
  .moderation-hub h1 {
    animation: none;
    background: none;
    -webkit-text-fill-color: var(--text-primary);
    color: var(--text-primary);
  }
  .hub-tab,
  .hub-tab:active,
  .reset-btn:active,
  .purge-btn:active,
  .apply-btn:active,
  .cancel-btn:active { transform: none; }
  .hub-tab::after { transition: none !important; }
  .tab-content { animation: none; }
  .ban-card {
    animation: none !important;
    opacity: 1;
    transform: none !important;
  }
  .ban-card:hover { transform: none; }
}

@media (max-width: 768px) {
  /* Onglets : passe en scroll horizontal au lieu de wrap (preserve l'animation
     active) avec width: 100% et overflow-x. */
  .hub-tabs {
    width: 100%;
    overflow-x: auto;
    -webkit-overflow-scrolling: touch;
    scrollbar-width: thin;
  }
  .hub-tab {
    padding: 8px 14px;
    font-size: 0.85rem;
    flex-shrink: 0;
    white-space: nowrap;
  }
  /* Filtres : empilage en colonne */
  .filters {
    flex-direction: column;
    gap: 8px;
  }
  .filters > * {
    width: 100%;
  }
}
</style>
