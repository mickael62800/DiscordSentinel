<script setup lang="ts">
import { ref, computed, onMounted, onBeforeUnmount } from "vue";
import { useToast } from "../../composables/useToast";
import { useInfractions } from "../../composables/useInfractions";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { useFormatDate } from "../../composables/useFormatDate";
import { useConfirm } from "../../composables/useConfirm";
import { useModeration } from "../../composables/useModeration";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useComponentVisibility } from "@/composables/useComponentVisibility";
import type { TableColumn, Infraction } from "../../types";
import { infractionTypeVariant } from "../../utils/variants";

import DataTable from "./DataTable.vue";
import AppBadge from "../atoms/AppBadge.vue";
import AppInput from "../atoms/AppInput.vue";
import AppSelect from "../atoms/AppSelect.vue";
import AppButton from "../atoms/AppButton.vue";
import LoadingState from "../atoms/LoadingState.vue";
import ErrorState from "../atoms/ErrorState.vue";
import ModerationActionModal from "./moderation-journal/ModerationActionModal.vue";

const emit = defineEmits<{
  /** Demande au parent de basculer vers Suivi → Notes & Preuves avec ce user. */
  "open-notes-evidence": [userId: string];
}>();

const { success, error: showError } = useToast();
const { formatShortDateTime: fmt } = useFormatDate();
const { confirm } = useConfirm();
const { visible: rbacVisible } = useComponentVisibility();

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
useRealtimeRefresh(["infraction_new", "strike_added"], fetchInfractions);

const { selectedGuildId } = useGuildSelector();
const { logAction } = useModeration();

const journalSearch = ref("");
const journalType = ref<string>("all");
const journalModerator = ref<string>("all");
const journalStatus = ref<"all" | "detection" | "action">("all");
const journalDateFrom = ref<string>("");
const journalDateTo = ref<string>("");
const hideDetections = ref(true);

const bulkMenuOpen = ref(false);
function closeBulkMenu() { bulkMenuOpen.value = false; }
onMounted(() => document.addEventListener("click", closeBulkMenu));
onBeforeUnmount(() => document.removeEventListener("click", closeBulkMenu));

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

  if (hideDetections.value) {
    // On se base sur la SOURCE (detection = proposition automod, action = validee),
    // pas sur le type : une proposition peut avoir un type "warn"/"mute"/"ban".
    // Par defaut on n'affiche QUE le valide (sanction appliquee via la carte/commande).
    rows = rows.filter((i) => (i.source ?? "detection") === "action");
  }

  // Masque les bans appliques (visibles dans l'onglet "Bannis actifs"), mais
  // garde les propositions de ban (source=detection) pour permettre l'application.
  rows = rows.filter((i) => {
    const isBan = i.infraction_type === "ban_permanent"
      || i.infraction_type === "ban_temp"
      || i.infraction_type === "ban";
    const isApplied = (i.source ?? "detection") === "action";
    return !(isBan && isApplied);
  });

  const q = journalSearch.value.trim().toLowerCase();
  if (q) {
    rows = rows.filter((i) =>
      [i.username, i.user_id, i.reason, i.infraction_type, i.moderator, i.server]
        .some((f) => String(f ?? "").toLowerCase().includes(q)),
    );
  }

  if (journalType.value !== "all") {
    rows = rows.filter((i) => i.infraction_type === journalType.value);
  }
  if (journalModerator.value !== "all") {
    rows = rows.filter((i) => i.moderator === journalModerator.value);
  }
  if (journalStatus.value !== "all") {
    rows = rows.filter((i) => (i.source ?? "detection") === journalStatus.value);
  }
  if (journalDateFrom.value) {
    const from = new Date(journalDateFrom.value).getTime();
    rows = rows.filter((i) => new Date(i.created_at).getTime() >= from);
  }
  if (journalDateTo.value) {
    const to = new Date(journalDateTo.value).getTime() + 86400000;
    rows = rows.filter((i) => new Date(i.created_at).getTime() < to);
  }

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

async function onDeleteInfraction(row: Record<string, unknown>) {
  const id = row.id as string;
  const source = (row.source as "detection" | "action" | undefined) ?? "detection";
  const actionType = String(row.infraction_type ?? "").toLowerCase();
  const isBan = source === "action" && actionType.startsWith("ban");
  const isMute = source === "action" && (actionType.startsWith("mute") || actionType === "timeout");

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
    /* toast deja affiche par le composable */
  }
}

// --- Action modal ---
const actionModalVisible = ref(false);

function openActionModal() {
  actionModalVisible.value = true;
}
</script>

<template>
  <div>
    <div class="card journal-toolbar">
      <div class="filters-grid">
        <div class="filter-field filter-search">
          <label>Recherche</label>
          <AppInput v-model="journalSearch" placeholder="Utilisateur, ID, raison, serveur…" />
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
        >Proposition</span>
        <span
          v-else-if="value === 'action'"
          class="source-chip applied"
          title="Sanction effectivement appliquee par un moderateur ou un bot"
        >Applique</span>
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
            @click.stop="emit('open-notes-evidence', String((row as Record<string, unknown>).user_id))"
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

    <ModerationActionModal
      :open="actionModalVisible"
      @close="actionModalVisible = false"
      @submitted="fetchInfractions"
    />
  </div>
</template>

<style scoped>
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
  .filters-grid { grid-template-columns: repeat(3, 1fr); }
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
.date-input:focus { border-color: var(--accent); }

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
.toggle-filter:hover { color: var(--text-primary); }

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
  transition: transform 0.28s cubic-bezier(0.34, 1.56, 0.64, 1), background 0.25s ease;
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
.switch input:checked + .slider::before { transform: translateX(16px); }
.switch input:focus-visible + .slider { outline: 2px solid var(--accent); outline-offset: 2px; }
.switch input:active + .slider::before { width: 18px; }

@media (prefers-reduced-motion: reduce) {
  .switch .slider, .switch .slider::before { transition: none; }
}

.reset-btn {
  background: linear-gradient(180deg,
    color-mix(in srgb, white 4%, var(--bg-card)),
    var(--bg-card));
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 7px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease, background 0.25s ease, border-color 0.2s ease, box-shadow 0.25s ease;
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
.reset-btn:active { transform: scale(0.97); transition-duration: 0.08s; }

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

.result-count {
  font-size: 12px;
  color: var(--text-secondary);
  margin-bottom: 12px;
  padding: 0 4px;
}
.result-count strong { color: var(--text-primary); font-weight: 700; font-size: 13px; }
.result-total { opacity: 0.7; }

.user-cell { display: flex; flex-direction: column; gap: 2px; }
.display-name {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-primary);
}
.display-name + .username { font-weight: 400; font-size: 12px; color: var(--text-secondary); }
.username { font-weight: 600; font-size: 14px; }
.user-id { font-size: 11px; color: var(--text-secondary); font-family: "JetBrains Mono", "Cascadia Code", monospace; }

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

.mono { font-family: "JetBrains Mono", "Cascadia Code", monospace; font-size: 12px; }

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
  background: linear-gradient(180deg,
    color-mix(in srgb, #57f287 8%, var(--bg-card)),
    var(--bg-card));
  color: #57f287;
  border: 1px solid color-mix(in srgb, #57f287 60%, var(--border));
  border-radius: 8px;
  padding: 8px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease, background 0.25s ease, border-color 0.2s ease, box-shadow 0.25s ease;
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
.apply-btn:active:not(:disabled) { transform: scale(0.97); transition-duration: 0.08s; }
.apply-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.apply-btn svg { width: 14px; height: 14px; flex-shrink: 0; }

.cancel-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  background: linear-gradient(180deg,
    color-mix(in srgb, var(--danger) 6%, var(--bg-card)),
    var(--bg-card));
  color: var(--danger);
  border: 1px solid color-mix(in srgb, var(--danger) 60%, var(--border));
  border-radius: 8px;
  padding: 8px 16px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: color 0.2s ease, background 0.25s ease, border-color 0.2s ease, box-shadow 0.25s ease;
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
.cancel-btn:active:not(:disabled) { transform: scale(0.97); transition-duration: 0.08s; }
.cancel-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.cancel-btn svg { width: 14px; height: 14px; flex-shrink: 0; }

@media (prefers-reduced-motion: reduce) {
  .reset-btn:active, .apply-btn:active, .cancel-btn:active { transform: none; }
}
</style>
