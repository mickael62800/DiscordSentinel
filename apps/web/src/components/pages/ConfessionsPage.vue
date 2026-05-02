<script setup lang="ts">
import { ref, computed, onMounted, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { useConfirm } from "@/composables/useConfirm";
import { useMyRole } from "@/composables/useMyRole";
import { useRealtimeRefresh } from "@/composables/useRealtimeRefresh";
import {
  confessionsService,
  type Confession,
  type ConfessionReply,
  type ConfessionReport,
} from "@/services/confessionsService";

const { selectedGuildId } = useGuildSelector();
const { success: toastOk, error: toastErr } = useToast();
const { confirm } = useConfirm();
const { isSuper, role } = useMyRole();
const isOwner = computed(() => isSuper.value || role.value === "owner");

const tab = ref<"confessions" | "reports">("confessions");
const showDeleted = ref(false);

const confessions = ref<Confession[]>([]);
const reports = ref<ConfessionReport[]>([]);
const loading = ref(false);

async function fetchAll() {
  if (!selectedGuildId.value) return;
  loading.value = true;
  try {
    const [c, r] = await Promise.all([
      confessionsService.list(selectedGuildId.value, showDeleted.value, 200),
      confessionsService.listReports(selectedGuildId.value, "pending", 100),
    ]);
    confessions.value = c;
    reports.value = r;
  } catch (e: unknown) {
    toastErr(`Echec chargement : ${(e as Error)?.message ?? e}`);
  } finally {
    loading.value = false;
  }
}
onMounted(fetchAll);
watch([selectedGuildId, showDeleted], fetchAll);

// Sync bidirectionnelle : si une confession est creee/editee/supprimee
// cote Discord (slash command admin) ou si un nouveau signalement arrive,
// on rafraichit automatiquement la page web. Permet aussi a 2 admins web
// d'avoir la meme vue en temps reel.
useRealtimeRefresh(
  [
    "confession_created",
    "confession_edited",
    "confession_deleted",
    "confession_reply_created",
    "confession_reply_deleted",
    "confession_report_created",
  ],
  fetchAll,
);

// ── Replies preview ────────────────────────────────────────────────────

const repliesTarget = ref<Confession | null>(null);
const replies = ref<ConfessionReply[]>([]);

async function showReplies(c: Confession) {
  repliesTarget.value = c;
  try {
    replies.value = await confessionsService.listReplies(c.id);
  } catch (e: unknown) {
    toastErr(`Echec replies : ${(e as Error)?.message ?? e}`);
    replies.value = [];
  }
}
function closeReplies() {
  repliesTarget.value = null;
  replies.value = [];
}

// ── Actions ────────────────────────────────────────────────────────────

async function deleteConfession(c: Confession) {
  const ok = await confirm({
    title: `Supprimer #${c.public_number}`,
    message: `Supprimer définitivement la confession #${c.public_number} ? Le message Discord sera aussi retiré.`,
  });
  if (!ok) return;
  try {
    await confessionsService.delete(c.id, "web-admin", "Supprimee par admin via web");
    toastOk(`Confession #${c.public_number} supprimee.`);
    await fetchAll();
  } catch (e: unknown) {
    toastErr(`Echec : ${(e as Error)?.message ?? e}`);
  }
}

async function deleteReply(r: ConfessionReply) {
  const ok = await confirm({ title: "Supprimer reply", message: "Confirmer ?" });
  if (!ok) return;
  try {
    await confessionsService.deleteReply(r.id, "web-admin");
    toastOk("Reply supprime.");
    if (repliesTarget.value) await showReplies(repliesTarget.value);
  } catch (e: unknown) {
    toastErr(`Echec : ${(e as Error)?.message ?? e}`);
  }
}

async function resolveReport(r: ConfessionReport, status: "resolved" | "dismissed") {
  try {
    await confessionsService.resolveReport(r.id, status, "web-admin");
    toastOk(`Signalement ${status === "resolved" ? "résolu" : "rejeté"}.`);
    await fetchAll();
  } catch (e: unknown) {
    toastErr(`Echec : ${(e as Error)?.message ?? e}`);
  }
}

function fmtDate(iso: string | null): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString("fr-FR");
}

function truncate(s: string, n = 100): string {
  return s.length > n ? s.slice(0, n) + "…" : s;
}
</script>

<template>
  <div class="confessions-page">
    <header class="page-head">
      <div>
        <h1>📝 Modération des confessions</h1>
        <p class="muted small">
          Confessions anonymes postées via /confess. Seul le owner voit l'auteur réel.
        </p>
      </div>
      <div class="actions">
        <label class="cb">
          <input v-model="showDeleted" type="checkbox" />
          <span>Afficher supprimées</span>
        </label>
      </div>
    </header>

    <div class="tabs">
      <button :class="['tab', { active: tab === 'confessions' }]" @click="tab = 'confessions'">
        Confessions ({{ confessions.length }})
      </button>
      <button :class="['tab', { active: tab === 'reports' }]" @click="tab = 'reports'">
        🚩 Signalements ({{ reports.length }})
      </button>
    </div>

    <div v-if="loading" class="muted">Chargement…</div>

    <!-- ── Onglet Confessions ── -->
    <table v-else-if="tab === 'confessions' && confessions.length > 0" class="data-table">
      <thead>
        <tr>
          <th>#</th>
          <th>Date</th>
          <th>Auteur</th>
          <th>Contenu</th>
          <th>État</th>
          <th class="actions-h">Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="c in confessions" :key="c.id" :class="{ deleted: c.deleted_at }">
          <td class="mono">#{{ c.public_number }}</td>
          <td class="small mono">{{ fmtDate(c.created_at) }}</td>
          <td class="small mono">
            <span v-if="isOwner">{{ c.author_user_id }}</span>
            <span v-else class="muted">[anonyme]</span>
          </td>
          <td class="small">
            {{ truncate(c.content, 100) }}
            <span v-if="c.edited_at" class="badge">éd.</span>
          </td>
          <td>
            <span v-if="c.deleted_at" class="badge danger">supprimée</span>
            <span v-else class="badge">active</span>
          </td>
          <td class="actions">
            <button class="btn-secondary xs" @click="showReplies(c)" title="Voir les replies">💬</button>
            <button v-if="!c.deleted_at" class="btn-danger xs" @click="deleteConfession(c)" title="Supprimer">🗑</button>
          </td>
        </tr>
      </tbody>
    </table>
    <p v-else-if="tab === 'confessions'" class="empty">Aucune confession.</p>

    <!-- ── Onglet Reports ── -->
    <table v-if="tab === 'reports' && reports.length > 0" class="data-table">
      <thead>
        <tr>
          <th>Date</th>
          <th>Cible</th>
          <th>Reporter</th>
          <th>Raison</th>
          <th>Status</th>
          <th class="actions-h">Actions</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="r in reports" :key="r.id">
          <td class="small mono">{{ fmtDate(r.created_at) }}</td>
          <td class="small mono">
            <span v-if="r.confession_id">Confession {{ r.confession_id.slice(0, 8) }}…</span>
            <span v-else-if="r.reply_id">Reply {{ r.reply_id.slice(0, 8) }}…</span>
          </td>
          <td class="small mono">{{ r.reporter_user_id }}</td>
          <td class="small">{{ truncate(r.reason, 80) }}</td>
          <td><span class="badge" :class="`status-${r.status}`">{{ r.status }}</span></td>
          <td class="actions">
            <button class="btn-secondary xs" @click="resolveReport(r, 'resolved')" title="Résoudre">✓</button>
            <button class="btn-secondary xs" @click="resolveReport(r, 'dismissed')" title="Rejeter">×</button>
          </td>
        </tr>
      </tbody>
    </table>
    <p v-else-if="tab === 'reports'" class="empty">Aucun signalement en attente 🎉</p>

    <!-- ── Modale replies ── -->
    <div v-if="repliesTarget" class="modal-overlay" @click.self="closeReplies">
      <div class="modal-card">
        <header class="modal-head">
          <h3>Replies de la confession #{{ repliesTarget.public_number }}</h3>
          <button class="modal-close" @click="closeReplies">×</button>
        </header>
        <div class="modal-body">
          <p v-if="replies.length === 0" class="muted">Aucune reply.</p>
          <div v-for="r in replies" :key="r.id" class="reply-row" :class="{ deleted: r.deleted_at }">
            <div class="reply-head">
              <strong>#{{ r.public_number }}</strong>
              <span v-if="r.is_anonymous" class="badge">anonyme</span>
              <span v-else class="badge">{{ r.author_user_id }}</span>
              <span class="muted small">{{ fmtDate(r.created_at) }}</span>
              <button v-if="!r.deleted_at" class="btn-danger xs" @click="deleteReply(r)">🗑</button>
            </div>
            <div class="reply-content">
              <span v-if="r.deleted_at" class="muted">[supprimée]</span>
              <span v-else>{{ r.content }}</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.confessions-page { padding: 0; }
.page-head { display: flex; justify-content: space-between; align-items: flex-start; margin-bottom: 16px; }
.page-head h1 { margin: 0; font-size: 24px; }
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
.mono { font-family: "JetBrains Mono", monospace; }
.empty { padding: 30px; text-align: center; color: var(--text-secondary); font-style: italic; }
.tabs { display: flex; gap: 4px; margin-bottom: 16px; border-bottom: 1px solid var(--border); }
.tab { background: transparent; border: 0; padding: 10px 16px; font-size: 13px; color: var(--text-secondary); cursor: pointer; border-bottom: 2px solid transparent; font-weight: 600; }
.tab:hover { color: var(--text-primary); }
.tab.active { color: var(--accent); border-bottom-color: var(--accent); }
.data-table { width: 100%; border-collapse: collapse; }
.data-table th, .data-table td { padding: 10px 12px; border-bottom: 1px solid var(--border); }
.data-table th { text-align: left; font-size: 11px; text-transform: uppercase; color: var(--text-secondary); letter-spacing: 0.5px; }
.data-table tr.deleted { opacity: 0.5; }
.data-table .actions-h, .data-table .actions { text-align: right; white-space: nowrap; }
.data-table .actions button { margin-left: 4px; }
.badge { display: inline-block; padding: 2px 6px; border-radius: 4px; background: var(--bg-secondary); color: var(--text-secondary); font-size: 10px; margin-left: 6px; text-transform: uppercase; letter-spacing: 0.5px; }
.badge.danger { background: rgba(231, 76, 60, 0.15); color: #e74c3c; }
.badge.status-pending { background: rgba(241, 196, 15, 0.15); color: #f1c40f; }
.badge.status-resolved { background: rgba(46, 204, 113, 0.15); color: #2ecc71; }
.badge.status-dismissed { background: rgba(138, 150, 168, 0.15); color: var(--text-secondary); }
.cb { display: inline-flex; align-items: center; gap: 6px; font-size: 12px; }

.btn-secondary, .btn-danger { padding: 4px 8px; border-radius: 6px; cursor: pointer; font-size: 11px; font-weight: 600; border: 1px solid var(--border); background: transparent; color: var(--text-primary); }
.btn-secondary:hover { background: var(--bg-hover); }
.btn-danger { color: var(--danger, #ef4444); border-color: color-mix(in srgb, var(--danger, #ef4444) 50%, var(--border)); }
.btn-danger:hover { background: color-mix(in srgb, var(--danger, #ef4444) 12%, transparent); }
.xs { padding: 4px 8px; }

.modal-overlay { position: fixed; inset: 0; z-index: 1000; background: rgba(0, 0, 0, 0.6); display: flex; align-items: center; justify-content: center; padding: 20px; }
.modal-card { background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px; width: 100%; max-width: 720px; max-height: 90vh; display: flex; flex-direction: column; }
.modal-head { display: flex; justify-content: space-between; align-items: center; padding: 16px 20px; border-bottom: 1px solid var(--border); }
.modal-head h3 { margin: 0; font-size: 16px; }
.modal-close { background: transparent; border: 0; cursor: pointer; font-size: 24px; line-height: 1; color: var(--text-secondary); padding: 0 6px; }
.modal-body { padding: 16px 20px; overflow-y: auto; flex: 1; }
.reply-row { padding: 8px 0; border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent); }
.reply-row.deleted { opacity: 0.5; }
.reply-head { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.reply-head .btn-danger { margin-left: auto; }
.reply-content { font-size: 13px; padding: 4px 0 4px 16px; word-wrap: break-word; }
</style>
