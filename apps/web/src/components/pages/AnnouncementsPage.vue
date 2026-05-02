<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import {
  announcementsService,
  type ScheduledAnnouncement,
  type AnnouncementRun,
  type RenderedAnnouncement,
  type CreateAnnouncementBody,
  type RecurrenceType,
  type ContentType,
} from "@/services/announcementsService";

const { selectedGuildId } = useGuildSelector();
const { success: toastOk, error: toastErr } = useToast();
const { confirm } = useConfirm();

const announcements = ref<ScheduledAnnouncement[]>([]);
const loading = ref(false);

async function fetchAll() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  try {
    announcements.value = await announcementsService.list(selectedGuildId.value);
  } catch (e: unknown) {
    toastErr(`Echec chargement annonces : ${(e as Error)?.message ?? e}`);
  } finally {
    loading.value = false;
  }
}
onMounted(fetchAll);
watch(selectedGuildId, fetchAll);

// ── Modale create/edit ──────────────────────────────────────────────────

interface FormState {
  open: boolean;
  mode: "create" | "edit";
  id: string | null;
  name: string;
  recurrence_type: RecurrenceType;
  recurrence_hour: number;
  recurrence_minute: number;
  recurrence_day_of_week: number | null;
  recurrence_day_of_month: number | null;
  scheduled_at: string;
  end_date: string;
  content_type: ContentType;
  content_text: string;
  embed_title: string;
  embed_color_hex: string;
  embed_image_url: string;
  embed_thumbnail_url: string;
  mention_everyone: boolean;
  mention_here: boolean;
  mention_role_ids: string;
  channel_ids: string;
}

const form = ref<FormState>(emptyForm());
const saving = ref(false);

function emptyForm(): FormState {
  return {
    open: false,
    mode: "create",
    id: null,
    name: "",
    recurrence_type: "daily",
    recurrence_hour: 12,
    recurrence_minute: 0,
    recurrence_day_of_week: 0,
    recurrence_day_of_month: 1,
    scheduled_at: "",
    end_date: "",
    content_type: "text",
    content_text: "",
    embed_title: "",
    embed_color_hex: "#5865f2",
    embed_image_url: "",
    embed_thumbnail_url: "",
    mention_everyone: false,
    mention_here: false,
    mention_role_ids: "",
    channel_ids: "",
  };
}

function openCreate() {
  form.value = { ...emptyForm(), open: true };
}

function openEdit(a: ScheduledAnnouncement) {
  form.value = {
    open: true,
    mode: "edit",
    id: a.id,
    name: a.name,
    recurrence_type: a.recurrence_type,
    recurrence_hour: a.recurrence_hour,
    recurrence_minute: a.recurrence_minute,
    recurrence_day_of_week: a.recurrence_day_of_week,
    recurrence_day_of_month: a.recurrence_day_of_month,
    scheduled_at: a.scheduled_at ? a.scheduled_at.slice(0, 16) : "",
    end_date: a.end_date ? a.end_date.slice(0, 16) : "",
    content_type: a.content_type,
    content_text: a.content_text,
    embed_title: a.embed_title ?? "",
    embed_color_hex: a.embed_color != null ? `#${a.embed_color.toString(16).padStart(6, "0")}` : "#5865f2",
    embed_image_url: a.embed_image_url ?? "",
    embed_thumbnail_url: a.embed_thumbnail_url ?? "",
    mention_everyone: a.mention_everyone,
    mention_here: a.mention_here,
    mention_role_ids: a.mention_role_ids.join(","),
    channel_ids: a.channel_ids.join(","),
  };
}

function closeForm() {
  form.value.open = false;
}

function buildBody(): CreateAnnouncementBody {
  const f = form.value;
  const ids = (s: string) =>
    s.split(",").map((x) => x.trim()).filter(Boolean);
  const colorInt = parseInt(f.embed_color_hex.replace("#", ""), 16);
  return {
    guild_id: selectedGuildId.value!,
    name: f.name,
    recurrence_type: f.recurrence_type,
    recurrence_hour: f.recurrence_hour,
    recurrence_minute: f.recurrence_minute,
    recurrence_day_of_week:
      f.recurrence_type === "weekly" ? f.recurrence_day_of_week : null,
    recurrence_day_of_month:
      f.recurrence_type === "monthly" ? f.recurrence_day_of_month : null,
    scheduled_at:
      f.recurrence_type === "once" && f.scheduled_at
        ? new Date(f.scheduled_at).toISOString()
        : null,
    end_date: f.end_date ? new Date(f.end_date).toISOString() : null,
    content_type: f.content_type,
    content_text: f.content_text,
    embed_title: f.content_type === "embed" ? f.embed_title || null : null,
    embed_color: f.content_type === "embed" && !Number.isNaN(colorInt) ? colorInt : null,
    embed_image_url: f.content_type === "embed" ? f.embed_image_url || null : null,
    embed_thumbnail_url: f.content_type === "embed" ? f.embed_thumbnail_url || null : null,
    mention_everyone: f.mention_everyone,
    mention_here: f.mention_here,
    mention_role_ids: ids(f.mention_role_ids),
    channel_ids: ids(f.channel_ids),
  };
}

async function saveForm() {
  if (!selectedGuildId.value) return;
  saving.value = true;
  try {
    const body = buildBody();
    if (form.value.mode === "create") {
      await announcementsService.create(body);
      toastOk("Annonce créée.");
    } else if (form.value.id) {
      // eslint-disable-next-line @typescript-eslint/no-unused-vars
      const { guild_id: _gid, ...rest } = body;
      await announcementsService.update(form.value.id, rest);
      toastOk("Annonce mise à jour.");
    }
    closeForm();
    await fetchAll();
  } catch (e: unknown) {
    toastErr(`Echec sauvegarde : ${(e as Error)?.message ?? e}`);
  } finally {
    saving.value = false;
  }
}

// ── Toggle on/off ──────────────────────────────────────────────────────

async function toggleEnabled(a: ScheduledAnnouncement) {
  try {
    await announcementsService.toggle(a.id, !a.enabled);
    await fetchAll();
  } catch (e: unknown) {
    toastErr(`Echec toggle : ${(e as Error)?.message ?? e}`);
  }
}

// ── Delete ─────────────────────────────────────────────────────────────

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

// ── Preview ────────────────────────────────────────────────────────────

const preview = ref<RenderedAnnouncement | null>(null);

async function showPreview(a: ScheduledAnnouncement) {
  try {
    preview.value = await announcementsService.preview(a.id);
  } catch (e: unknown) {
    toastErr(`Echec preview : ${(e as Error)?.message ?? e}`);
  }
}

function closePreview() {
  preview.value = null;
}

// ── Historique ─────────────────────────────────────────────────────────

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

// ── Helpers UI ─────────────────────────────────────────────────────────

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

const formCanSave = computed(() => {
  const f = form.value;
  if (!f.name.trim()) return false;
  if (!f.channel_ids.trim()) return false;
  if (f.recurrence_type === "once" && !f.scheduled_at) return false;
  return true;
});
</script>

<template>
  <div class="announcements-page">
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
    <div v-else-if="announcements.length === 0" class="empty">
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
            <button class="btn-danger xs" @click="removeAnnouncement(a)" title="Supprimer">🗑</button>
          </td>
        </tr>
      </tbody>
    </table>

    <!-- ── Modale create / edit ── -->
    <div v-if="form.open" class="modal-overlay" @click.self="closeForm">
      <div class="modal-card large">
        <header class="modal-head">
          <h3>{{ form.mode === "create" ? "Nouvelle annonce" : "Modifier l'annonce" }}</h3>
          <button class="modal-close" @click="closeForm">×</button>
        </header>
        <div class="modal-body">
          <div class="grid-2">
            <label>
              Nom *
              <input v-model="form.name" type="text" placeholder="ex: Rappel Tournoi du dimanche" />
            </label>
            <label>
              Type de récurrence
              <select v-model="form.recurrence_type">
                <option value="once">Ponctuelle (une fois)</option>
                <option value="daily">Quotidienne</option>
                <option value="weekly">Hebdomadaire</option>
                <option value="monthly">Mensuelle</option>
              </select>
            </label>
          </div>

          <div class="grid-2">
            <label>
              Heure (UTC)
              <input v-model.number="form.recurrence_hour" type="number" min="0" max="23" />
            </label>
            <label>
              Minute
              <input v-model.number="form.recurrence_minute" type="number" min="0" max="59" />
            </label>
          </div>

          <label v-if="form.recurrence_type === 'weekly'">
            Jour de la semaine
            <select v-model.number="form.recurrence_day_of_week">
              <option v-for="(d, i) in dowLabels" :key="i" :value="i">{{ d }}</option>
            </select>
          </label>

          <label v-if="form.recurrence_type === 'monthly'">
            Jour du mois (1-31, clamp si mois plus court)
            <input v-model.number="form.recurrence_day_of_month" type="number" min="1" max="31" />
          </label>

          <label v-if="form.recurrence_type === 'once'">
            Date et heure de l'annonce
            <input v-model="form.scheduled_at" type="datetime-local" />
          </label>

          <label>
            Date de fin (optionnelle, vide = indéfini)
            <input v-model="form.end_date" type="datetime-local" />
          </label>

          <hr class="sep" />

          <label>
            Format
            <select v-model="form.content_type">
              <option value="text">Texte simple</option>
              <option value="embed">Embed riche (carte)</option>
            </select>
          </label>

          <template v-if="form.content_type === 'embed'">
            <div class="grid-2">
              <label>
                Titre embed
                <input v-model="form.embed_title" type="text" />
              </label>
              <label>
                Couleur
                <input v-model="form.embed_color_hex" type="color" />
              </label>
            </div>
            <label>
              URL image (grande, en bas)
              <input v-model="form.embed_image_url" type="url" placeholder="https://..." />
            </label>
            <label>
              URL thumbnail (petite, à droite)
              <input v-model="form.embed_thumbnail_url" type="url" placeholder="https://..." />
            </label>
          </template>

          <label>
            {{ form.content_type === "embed" ? "Description (variables : {date} {day_name} {time} ...)" : "Contenu (variables : {date} {day_name} {time} ...)" }}
            <textarea v-model="form.content_text" rows="5"></textarea>
          </label>

          <hr class="sep" />

          <div class="checkbox-row">
            <label class="cb">
              <input v-model="form.mention_everyone" type="checkbox" />
              <span>Mentionner @everyone</span>
            </label>
            <label class="cb">
              <input v-model="form.mention_here" type="checkbox" />
              <span>Mentionner @here</span>
            </label>
          </div>

          <label>
            IDs de rôles à mentionner (séparés par virgule)
            <input v-model="form.mention_role_ids" type="text" placeholder="123456789012345678,..." />
          </label>

          <label>
            IDs des salons cibles * (séparés par virgule, au moins 1)
            <input v-model="form.channel_ids" type="text" placeholder="123456789012345678,..." />
          </label>
        </div>
        <footer class="modal-foot">
          <button class="btn-secondary" :disabled="saving" @click="closeForm">Annuler</button>
          <button class="btn-primary" :disabled="!formCanSave || saving" @click="saveForm">
            {{ saving ? "Enregistrement…" : "Enregistrer" }}
          </button>
        </footer>
      </div>
    </div>

    <!-- ── Modale preview ── -->
    <div v-if="preview" class="modal-overlay" @click.self="closePreview">
      <div class="modal-card">
        <header class="modal-head">
          <h3>👁 Aperçu</h3>
          <button class="modal-close" @click="closePreview">×</button>
        </header>
        <div class="modal-body preview-body">
          <p v-if="preview.mentions_prefix" class="prev-mentions">{{ preview.mentions_prefix }}</p>
          <div v-if="preview.embed" class="prev-embed" :style="{ borderLeftColor: preview.embed.color != null ? '#' + preview.embed.color.toString(16).padStart(6, '0') : '#5865f2' }">
            <h4 v-if="preview.embed.title">{{ preview.embed.title }}</h4>
            <p class="prev-desc">{{ preview.embed.description }}</p>
            <img v-if="preview.embed.thumbnail_url" :src="preview.embed.thumbnail_url" class="prev-thumb" />
            <img v-if="preview.embed.image_url" :src="preview.embed.image_url" class="prev-img" />
          </div>
          <p v-else class="prev-text">{{ preview.content_text }}</p>
          <p class="muted small">
            Sera publié sur {{ preview.channel_ids.length }} salon{{ preview.channel_ids.length > 1 ? "s" : "" }}.
          </p>
        </div>
      </div>
    </div>

    <!-- ── Modale historique ── -->
    <div v-if="runsTarget" class="modal-overlay" @click.self="closeRuns">
      <div class="modal-card">
        <header class="modal-head">
          <h3>📜 Historique — {{ runsTarget.name }}</h3>
          <button class="modal-close" @click="closeRuns">×</button>
        </header>
        <div class="modal-body">
          <table v-if="runs.length > 0" class="data-table">
            <thead>
              <tr>
                <th>Date</th>
                <th>Statut</th>
                <th>Salons</th>
                <th>Erreur</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="r in runs" :key="r.id">
                <td class="small mono">{{ fmtDate(r.ran_at) }}</td>
                <td>
                  <span class="badge" :class="`status-${r.status}`">{{ r.status }}</span>
                </td>
                <td class="small">
                  {{ r.channels_posted.filter((c) => c.success).length }}/{{ r.channels_posted.length }} OK
                </td>
                <td class="small muted">{{ r.error ?? "—" }}</td>
              </tr>
            </tbody>
          </table>
          <p v-else class="muted">Aucun envoi pour le moment.</p>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.announcements-page { padding: 0; }
.page-head { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 24px; }
.page-head h1 { margin: 0; font-size: 24px; }
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
.mono { font-family: "JetBrains Mono", monospace; }
.empty { padding: 40px; text-align: center; color: var(--text-secondary); font-style: italic; }
.data-table { width: 100%; border-collapse: collapse; }
.data-table th, .data-table td { padding: 10px 12px; border-bottom: 1px solid var(--border); }
.data-table th { text-align: left; font-size: 11px; text-transform: uppercase; color: var(--text-secondary); letter-spacing: 0.5px; }
.data-table tr.off { opacity: 0.5; }
.data-table .actions-h, .data-table .actions { text-align: right; white-space: nowrap; }
.data-table .actions button { margin-left: 4px; }
.badge { display: inline-block; padding: 2px 6px; border-radius: 4px; background: var(--bg-secondary); color: var(--text-secondary); font-size: 10px; margin-left: 6px; text-transform: uppercase; letter-spacing: 0.5px; }
.badge.status-success { background: rgba(46, 204, 113, 0.15); color: #2ecc71; }
.badge.status-partial { background: rgba(241, 196, 15, 0.15); color: #f1c40f; }
.badge.status-error { background: rgba(231, 76, 60, 0.15); color: #e74c3c; }
.badge.status-pending { background: rgba(138, 150, 168, 0.15); color: var(--text-secondary); }

.toggle { padding: 4px 10px; font-size: 11px; font-weight: 700; border: 1px solid var(--border); border-radius: 6px; background: transparent; color: var(--text-secondary); cursor: pointer; }
.toggle.on { background: rgba(46, 204, 113, 0.18); color: #2ecc71; border-color: #2ecc71; }

.btn-primary, .btn-secondary, .btn-danger { padding: 8px 14px; border-radius: 8px; cursor: pointer; font-size: 13px; font-weight: 600; border: 1px solid var(--border); }
.btn-primary { background: var(--accent); color: white; border-color: var(--accent); }
.btn-primary:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-secondary { background: transparent; color: var(--text-primary); }
.btn-secondary:hover:not(:disabled) { background: var(--bg-hover); }
.btn-danger { background: transparent; color: var(--danger, #ef4444); border-color: color-mix(in srgb, var(--danger, #ef4444) 50%, var(--border)); }
.btn-danger:hover:not(:disabled) { background: color-mix(in srgb, var(--danger, #ef4444) 12%, transparent); }
.xs { padding: 4px 8px; font-size: 11px; }

/* Modale generique */
.modal-overlay {
  position: fixed; inset: 0; z-index: 1000;
  background: rgba(0, 0, 0, 0.6);
  display: flex; align-items: center; justify-content: center;
  padding: 20px;
}
.modal-card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  width: 100%; max-width: 560px;
  max-height: 90vh;
  display: flex; flex-direction: column;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
}
.modal-card.large { max-width: 800px; }
.modal-head { display: flex; justify-content: space-between; align-items: center; padding: 16px 20px; border-bottom: 1px solid var(--border); }
.modal-head h3 { margin: 0; font-size: 16px; }
.modal-close { background: transparent; border: 0; cursor: pointer; font-size: 24px; line-height: 1; color: var(--text-secondary); padding: 0 6px; }
.modal-body { padding: 20px; overflow-y: auto; flex: 1; }
.modal-foot { display: flex; justify-content: flex-end; gap: 10px; padding: 14px 20px; border-top: 1px solid var(--border); }
.modal-body label { display: block; font-size: 13px; font-weight: 600; margin-bottom: 14px; }
.modal-body label > input,
.modal-body label > textarea,
.modal-body label > select {
  margin-top: 4px;
  width: 100%;
  padding: 8px 10px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: 13px;
  box-sizing: border-box;
}
.modal-body label > input[type="color"] { padding: 2px; height: 36px; }
.grid-2 { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
.checkbox-row { display: flex; gap: 16px; flex-wrap: wrap; margin-bottom: 14px; }
.cb { display: inline-flex; align-items: center; gap: 6px; font-weight: 500; margin-bottom: 0; }
.cb input { width: auto; margin: 0; }
.sep { border: 0; border-top: 1px solid var(--border); margin: 18px 0; }

/* Preview */
.preview-body { display: flex; flex-direction: column; gap: 10px; }
.prev-mentions { font-weight: 600; color: var(--accent); margin: 0; }
.prev-embed {
  background: var(--bg-secondary);
  border-left: 4px solid var(--accent);
  border-radius: 4px;
  padding: 12px;
}
.prev-embed h4 { margin: 0 0 6px 0; font-size: 14px; }
.prev-desc { white-space: pre-wrap; margin: 0; font-size: 13px; }
.prev-text { white-space: pre-wrap; margin: 0; font-size: 13px; }
.prev-img { max-width: 100%; border-radius: 6px; margin-top: 8px; }
.prev-thumb { max-width: 80px; max-height: 80px; border-radius: 6px; float: right; margin-left: 10px; }
</style>
