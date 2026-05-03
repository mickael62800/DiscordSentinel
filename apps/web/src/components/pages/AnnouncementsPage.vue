<script setup lang="ts">
import { ref, onMounted, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import {
  announcementsService,
  type ScheduledAnnouncement,
  type AnnouncementRun,
  type RenderedAnnouncement,
} from "@/services/announcementsService";
import { guildsService, type DiscordTextChannel } from "@/services/guildsService";
import { discordRolesService } from "@/services/discordRolesService";
import type { DiscordRole } from "@/types";
import AnnouncementFormModal from "../organisms/AnnouncementFormModal.vue";
import AnnouncementPreviewModal from "../organisms/AnnouncementPreviewModal.vue";
import AnnouncementRunsModal from "../organisms/AnnouncementRunsModal.vue";

const { selectedGuildId } = useGuildSelector();
const { success: toastOk, error: toastErr } = useToast();
const { confirm } = useConfirm();

const announcements = ref<ScheduledAnnouncement[]>([]);
const loading = ref(false);
const channels = ref<DiscordTextChannel[]>([]);
const roles = ref<DiscordRole[]>([]);

async function fetchAll() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  try {
    const [list, ch, ro] = await Promise.all([
      announcementsService.list(selectedGuildId.value),
      guildsService.getTextChannels(selectedGuildId.value).catch(() => []),
      discordRolesService.getAll(selectedGuildId.value).catch(() => []),
    ]);
    announcements.value = list;
    channels.value = ch;
    roles.value = ro;
  } catch (e: unknown) {
    toastErr(`Echec chargement annonces : ${(e as Error)?.message ?? e}`);
  } finally {
    loading.value = false;
  }
}
onMounted(fetchAll);
watch(selectedGuildId, fetchAll);

// Form modal
const formOpen = ref(false);
const formTarget = ref<ScheduledAnnouncement | null>(null);

function openCreate() {
  formTarget.value = null;
  formOpen.value = true;
}

function openEdit(a: ScheduledAnnouncement) {
  formTarget.value = a;
  formOpen.value = true;
}

function closeForm() {
  formOpen.value = false;
}

async function toggleEnabled(a: ScheduledAnnouncement) {
  try {
    await announcementsService.toggle(a.id, !a.enabled);
    await fetchAll();
  } catch (e: unknown) {
    toastErr(`Echec toggle : ${(e as Error)?.message ?? e}`);
  }
}

async function removeAnnouncement(a: ScheduledAnnouncement) {
  const ok = await confirm({
    title: `Supprimer ${a.name}`,
    message: `Supprimer définitivement l'annonce "${a.name}" ? L'historique sera également effacé.`,
  });
  if (!ok) return;
  try {
    await announcementsService.delete(a.id);
    toastOk("Annonce supprimée.");
    await fetchAll();
  } catch (e: unknown) {
    toastErr(`Echec suppression : ${(e as Error)?.message ?? e}`);
  }
}

// Preview modal
const preview = ref<RenderedAnnouncement | null>(null);
async function showPreview(a: ScheduledAnnouncement) {
  try {
    preview.value = await announcementsService.preview(a.id);
  } catch (e: unknown) {
    toastErr(`Echec preview : ${(e as Error)?.message ?? e}`);
  }
}
function closePreview() { preview.value = null; }

// Runs modal
const runsTarget = ref<ScheduledAnnouncement | null>(null);
const runs = ref<AnnouncementRun[]>([]);
async function showRuns(a: ScheduledAnnouncement) {
  runsTarget.value = a;
  try {
    runs.value = await announcementsService.listRuns(a.id, 50);
  } catch (e: unknown) {
    toastErr(`Echec chargement runs : ${(e as Error)?.message ?? e}`);
    runs.value = [];
  }
}
function closeRuns() {
  runsTarget.value = null;
  runs.value = [];
}

const dowLabels = ["Lundi", "Mardi", "Mercredi", "Jeudi", "Vendredi", "Samedi", "Dimanche"];

function recurrenceLabel(a: ScheduledAnnouncement): string {
  const time = `${a.recurrence_hour.toString().padStart(2, "0")}:${a.recurrence_minute
    .toString()
    .padStart(2, "0")}`;
  switch (a.recurrence_type) {
    case "once":
      return `Une fois — ${a.scheduled_at ? new Date(a.scheduled_at).toLocaleString("fr-FR") : "?"}`;
    case "daily":
      return `Quotidien à ${time}`;
    case "weekly":
      return `Tous les ${dowLabels[a.recurrence_day_of_week ?? 0]} à ${time}`;
    case "monthly":
      return `Le ${a.recurrence_day_of_month ?? "?"} de chaque mois à ${time}`;
  }
}

function fmtDate(iso: string | null): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString("fr-FR");
}
</script>

<template>
  <div class="announcements-page page--wide">
    <header class="page-head">
      <div>
        <h1>📣 Annonces planifiées</h1>
        <p class="muted small">
          Messages Discord postés automatiquement (ponctuel, quotidien, hebdo, mensuel).
        </p>
      </div>
      <button class="btn-primary" :disabled="!selectedGuildId" @click="openCreate">
        + Nouvelle annonce
      </button>
    </header>

    <div v-if="loading" class="muted">Chargement…</div>
    <div v-else-if="announcements.length === 0" class="empty-state">
      Aucune annonce. Crée la première avec le bouton ci-dessus.
    </div>
    <table v-else class="data-table">
      <thead>
        <tr>
          <th>Nom</th>
          <th>Récurrence</th>
          <th>Prochain envoi</th>
          <th>État</th>
          <th>Salons</th>
          <th class="actions-h">Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="a in announcements" :key="a.id" :class="{ off: !a.enabled }">
          <td>
            <strong>{{ a.name }}</strong>
            <span v-if="a.content_type === 'embed'" class="badge">Embed</span>
          </td>
          <td class="small">{{ recurrenceLabel(a) }}</td>
          <td class="small mono">{{ fmtDate(a.next_run_at) }}</td>
          <td>
            <button class="toggle" :class="{ on: a.enabled }" @click="toggleEnabled(a)">
              {{ a.enabled ? "ON" : "OFF" }}
            </button>
          </td>
          <td class="small">{{ a.channel_ids.length }} salon{{ a.channel_ids.length > 1 ? "s" : "" }}</td>
          <td class="actions">
            <button class="btn-secondary xs" @click="showPreview(a)" title="Aperçu">👁</button>
            <button class="btn-secondary xs" @click="showRuns(a)" title="Historique">📜</button>
            <button class="btn-secondary xs" @click="openEdit(a)" title="Editer">✎</button>
            <button class="btn-secondary xs danger" @click="removeAnnouncement(a)" title="Supprimer">🗑</button>
          </td>
        </tr>
      </tbody>
    </table>

    <AnnouncementFormModal
      :visible="formOpen"
      :target="formTarget"
      :channels="channels"
      :roles="roles"
      :guild-id="selectedGuildId ?? ''"
      @close="closeForm"
      @saved="fetchAll"
    />

    <AnnouncementPreviewModal :preview="preview" @close="closePreview" />

    <AnnouncementRunsModal :target="runsTarget" :runs="runs" @close="closeRuns" />
  </div>
</template>

<style scoped>
.announcements-page { padding: 0; }
.page-head { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px; }
.page-head h1 { margin: 0; font-size: 24px; }
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
.mono { font-family: "JetBrains Mono", monospace; }

/* Table : utilise .data-table de global.css mais on override quelques specifiques */
.data-table th, .data-table td { padding: 10px 12px !important; }
.data-table tr.off { opacity: 0.5; }
.data-table .actions button { margin-left: 4px; }

.badge {
  display: inline-block;
  padding: 2px 6px;
  border-radius: 4px;
  background: var(--bg-secondary);
  color: var(--text-secondary);
  font-size: 10px;
  margin-left: 6px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.toggle {
  padding: 4px 10px;
  font-size: 11px;
  font-weight: 700;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  cursor: pointer;
}
.toggle.on {
  background: rgba(46, 204, 113, 0.18);
  color: #2ecc71;
  border-color: #2ecc71;
}

.btn-primary {
  padding: 8px 14px;
  border-radius: 8px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  border: 1px solid var(--accent);
  background: var(--accent);
  color: white;
}
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
