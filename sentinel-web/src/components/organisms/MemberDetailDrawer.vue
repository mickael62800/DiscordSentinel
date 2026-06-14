<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useMembers } from "../../composables/useMembers";
import { useFormatDate } from "../../composables/useFormatDate";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import { safeImageUrl } from "../../utils/safeUrl";
import AppBadge from "../atoms/AppBadge.vue";
import AppTabs from "../molecules/AppTabs.vue";
import PaginationBar from "../molecules/PaginationBar.vue";

const { success, error: showError } = useToast();
const { confirm: confirmDialog } = useConfirm();
const { formatShortDateTime: fmt } = useFormatDate();

const {
  selectedMember,
  loadingSummary,
  dossier,
  dossierLoading,
  activityTimeline,
  isWatched,
  selectMember,
  fetchDossier,
  addToWatch,
  removeFromWatch,
  resetMember,
  closeMember,
} = useMembers();

const detailTab = ref<"profil" | "surveillance">("profil");

const detailTabsItems = [
  { key: "profil", label: "Profil" },
  { key: "surveillance", label: "Surveillance" },
];

// Reset au sous-onglet "Profil" a chaque selection d'un nouveau membre.
watch(
  () => selectedMember.value?.member.user_id,
  (id, prev) => { if (id && id !== prev) detailTab.value = "profil"; },
);

watch(detailTab, async (tab) => {
  if (!selectedMember.value) return;
  const userId = selectedMember.value.member.user_id;
  if (tab === "surveillance") await fetchDossier(userId);
});

const watchAction = ref(false);
const resetting = ref(false);

async function toggleWatch() {
  if (!selectedMember.value) return;
  watchAction.value = true;
  try {
    await addToWatch(selectedMember.value.member.user_id, selectedMember.value.member.username);
    await fetchDossier(selectedMember.value.member.user_id);
    success("Membre mis en surveillance");
  } catch {
    showError("Impossible de mettre en surveillance (deja surveille ?)");
  } finally {
    watchAction.value = false;
  }
}

async function unwatch() {
  if (!selectedMember.value) return;
  const ok = await confirmDialog({
    title: "Retirer de la surveillance",
    message: `Retirer ${selectedMember.value.member.username} de la surveillance ?`,
  });
  if (!ok) return;
  watchAction.value = true;
  try {
    await removeFromWatch(selectedMember.value.member.user_id);
    dossier.value = null;
    await fetchDossier(selectedMember.value.member.user_id);
    success("Membre retire de la surveillance");
  } catch (e) {
    console.error("Erreur retrait surveillance:", e);
    showError("Erreur lors du retrait de la surveillance");
  } finally {
    watchAction.value = false;
  }
}

async function handleReset() {
  if (!selectedMember.value) return;
  const member = selectedMember.value.member;
  const username = member.display_name || member.username;
  const ok1 = await confirmDialog({
    title: "⚠️ Réinitialiser tout",
    message:
      `Supprimer DÉFINITIVEMENT toutes les données pour ${username} ?\n\n` +
      "Cela efface :\n" +
      "• Infractions\n" +
      "• Actions de modération (warns/mutes/bans)\n" +
      "• Strikes\n" +
      "• Notes modérateurs\n" +
      "• Surveillance manuelle\n" +
      "• Rappels de sanction\n" +
      "• Logs d'activité (surveillance détaillée)\n" +
      "• Statistiques utilisateur (messages, vocal)\n" +
      "• Sessions vocales détaillées\n\n" +
      "→ Le membre repart vraiment de zéro, page blanche.\n\n" +
      "Cette action est IRRÉVERSIBLE.",
  });
  if (!ok1) return;
  const ok2 = await confirmDialog({
    title: "Derniere confirmation",
    message: `Vraiment reinitialiser ${username} ? Tape OK pour confirmer.`,
  });
  if (!ok2) return;
  resetting.value = true;
  try {
    const totals = await resetMember(member.user_id);
    const summary = Object.entries(totals)
      .filter(([, n]) => n > 0)
      .map(([k, n]) => `${k}: ${n}`)
      .join(", ");
    success(`Membre reinitialise (${summary || "rien a supprimer"}).`);
    await selectMember(member.user_id);
    if (detailTab.value === "surveillance") {
      await fetchDossier(member.user_id);
    }
  } catch (e) {
    console.error("Erreur reset membre:", e);
    showError("Erreur lors de la reinitialisation du membre");
  } finally {
    resetting.value = false;
  }
}

// Helpers
function formatDate(date: string | null): string {
  if (!date) return "-";
  return new Date(date).toLocaleDateString("fr-FR", { day: "numeric", month: "short", year: "numeric" });
}

function formatDuration(seconds: number): string {
  const h = Math.floor(seconds / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function rolesCount(roles: unknown): number {
  return Array.isArray(roles) ? roles.length : 0;
}

// ── Activity timeline ────────────────
const URL_RE = /https?:\/\/[^\s]+/i;

const activityTypeFilter = ref<"all" | "text" | "vocal" | "other">("all");
const activityDateFrom = ref<string>("");
const activityDateTo = ref<string>("");

const TEXT_EVENTS = ["message_sent", "message_edited", "message_deleted"];
const VOCAL_EVENTS = ["voice_join", "voice_leave", "voice_move"];

function eventCategory(t: string): "text" | "vocal" | "other" {
  if (TEXT_EVENTS.includes(t)) return "text";
  if (VOCAL_EVENTS.includes(t)) return "vocal";
  return "other";
}

const filteredActivity = computed(() => {
  const list = activityTimeline.value ?? [];
  const fromTs = activityDateFrom.value ? new Date(activityDateFrom.value).getTime() : null;
  const toTs = activityDateTo.value ? new Date(activityDateTo.value).getTime() + 86400000 : null;
  return list.filter((e) => {
    if (activityTypeFilter.value !== "all" && eventCategory(e.event_type) !== activityTypeFilter.value) return false;
    const ts = new Date(e.created_at).getTime();
    if (fromTs !== null && ts < fromTs) return false;
    if (toTs !== null && ts >= toTs) return false;
    return true;
  });
});

function resetActivityFilters() {
  activityTypeFilter.value = "all";
  activityDateFrom.value = "";
  activityDateTo.value = "";
  activityPage.value = 1;
}

const activityPage = ref(1);
const activityPerPage = ref(25);
const activityTotalPages = computed(() => Math.max(1, Math.ceil(filteredActivity.value.length / activityPerPage.value)));
const activityPageRows = computed(() => {
  const start = (activityPage.value - 1) * activityPerPage.value;
  return filteredActivity.value.slice(start, start + activityPerPage.value);
});

watch([activityTypeFilter, activityDateFrom, activityDateTo], () => { activityPage.value = 1; });
watch(activityTotalPages, (n) => { if (activityPage.value > n) activityPage.value = n; });

function activityCount(type: string): number {
  return (activityTimeline.value ?? []).filter((e) => e.event_type === type).length;
}
function activityLinksCount(): number {
  return (activityTimeline.value ?? []).filter((e) => typeof e.content === "string" && URL_RE.test(e.content)).length;
}
function activityAttachmentsCount(): number {
  return (activityTimeline.value ?? []).filter((e) => {
    const m = e.metadata as Record<string, unknown> | null | undefined;
    const att = m?.attachments;
    return Array.isArray(att) && att.length > 0;
  }).length;
}
function activityLabel(t: string): string {
  return ({
    message_sent: "Message",
    message_edited: "Edite",
    message_deleted: "Supprime",
    voice_join: "Entree vocal",
    voice_leave: "Sortie vocal",
    voice_move: "Move vocal",
    roles_changed: "Roles",
    nickname_changed: "Pseudo",
    avatar_changed: "Avatar",
    member_join: "Arrivee",
    member_leave: "Depart",
  } as Record<string, string>)[t] ?? t;
}
function activityVariant(t: string): "default" | "warning" | "danger" | "info" | "success" {
  if (t === "message_deleted") return "danger";
  if (t === "message_edited" || t === "voice_leave" || t === "member_leave") return "warning";
  if (t === "member_join" || t === "voice_join") return "success";
  if (t.startsWith("voice_") || t === "message_sent") return "info";
  return "default";
}

type Meta = Record<string, unknown> | null | undefined;
function metaStr(m: Meta, ...keys: string[]): string | null {
  if (!m) return null;
  for (const k of keys) {
    const v = m[k];
    if (typeof v === "string" && v.trim() !== "") return v;
  }
  return null;
}
function metaArr(m: Meta, key: string): string[] {
  if (!m) return [];
  const v = m[key];
  return Array.isArray(v) ? v.filter((x): x is string => typeof x === "string") : [];
}

function voiceChannelLabel(evt: { channel_name?: string | null; channel_id?: string | null; metadata: Meta }): string {
  const name = evt.channel_name || metaStr(evt.metadata, "channel_name", "voice_channel_name");
  const id = evt.channel_id || metaStr(evt.metadata, "channel_id", "voice_channel_id");
  if (name && id) return `🔊 ${name} (${id})`;
  if (name) return `🔊 ${name}`;
  if (id) return `🔊 ${id}`;
  return "";
}

function editedBeforeAfter(evt: { content: string | null; metadata: Meta }): { before: string | null; after: string | null } {
  const before = metaStr(evt.metadata, "old_content", "before", "content_before", "previous_content");
  const after = metaStr(evt.metadata, "new_content", "after", "content_after") || evt.content;
  return { before, after };
}

function rolesDiff(evt: { metadata: Meta }): { added: string[]; removed: string[] } {
  const added = metaArr(evt.metadata, "added").length > 0
    ? metaArr(evt.metadata, "added")
    : metaArr(evt.metadata, "roles_added");
  const removed = metaArr(evt.metadata, "removed").length > 0
    ? metaArr(evt.metadata, "removed")
    : metaArr(evt.metadata, "roles_removed");
  return { added, removed };
}

function profileDiff(evt: { content: string | null; metadata: Meta }): { before: string | null; after: string | null } {
  const before = metaStr(evt.metadata, "old", "old_value", "before", "from", "old_nickname", "old_username");
  const after =
    metaStr(evt.metadata, "new", "new_value", "after", "to", "new_nickname", "new_username") || evt.content;
  return { before, after };
}

function avatarDiff(evt: { metadata: Meta }): { before: string | null; after: string | null } {
  return {
    before: metaStr(evt.metadata, "old_avatar_url", "old_avatar", "before"),
    after: metaStr(evt.metadata, "new_avatar_url", "new_avatar", "after"),
  };
}

// ── Surveillance enrichie ──
const NOW_MS = () => Date.now();
const ONE_DAY = 86_400_000;

function activityWithin(days: number) {
  const list = activityTimeline.value ?? [];
  if (days <= 0) return list;
  const since = NOW_MS() - days * ONE_DAY;
  return list.filter((e) => new Date(e.created_at).getTime() >= since);
}

function countByCategory(days: number, cat: "text" | "vocal" | "other"): number {
  return activityWithin(days).filter((e) => eventCategory(e.event_type) === cat).length;
}

function voiceHours(days: number): number {
  let total = 0;
  for (const e of activityWithin(days)) {
    if (e.event_type !== "voice_leave" && e.event_type !== "voice_move") continue;
    const m = e.metadata as Meta;
    const d = m?.duration_secs;
    if (typeof d === "number") total += d;
    else if (typeof d === "string") total += parseInt(d, 10) || 0;
  }
  return Math.round((total / 3600) * 10) / 10;
}

function attachmentCounts() {
  const list = activityTimeline.value ?? [];
  let images = 0, videos = 0, files = 0, links = 0;
  for (const e of list) {
    if (typeof e.content === "string" && URL_RE.test(e.content)) links++;
    const m = e.metadata as Meta;
    const att = m?.attachments;
    if (Array.isArray(att)) {
      for (const a of att) {
        if (typeof a === "string") {
          if (/\.(png|jpg|jpeg|gif|webp)/i.test(a)) images++;
          else if (/\.(mp4|webm|mov)/i.test(a)) videos++;
          else files++;
        } else if (a && typeof a === "object") {
          const ct = (a as Record<string, unknown>).content_type as string | undefined;
          if (ct?.startsWith("image/")) images++;
          else if (ct?.startsWith("video/")) videos++;
          else files++;
        }
      }
    }
  }
  return { images, videos, files, links };
}

function topChannels(limit = 5): Array<{ name: string; id: string; count: number }> {
  const counts = new Map<string, { name: string; id: string; count: number }>();
  for (const e of activityTimeline.value ?? []) {
    if (!TEXT_EVENTS.includes(e.event_type)) continue;
    const id = e.channel_id ?? "";
    const name = e.channel_name ?? id ?? "?";
    if (!id) continue;
    const cur = counts.get(id) ?? { name, id, count: 0 };
    cur.count++;
    counts.set(id, cur);
  }
  return [...counts.values()].sort((a, b) => b.count - a.count).slice(0, limit);
}

function topVoiceCompanions(limit = 5): Array<{ user_id: string; username: string; count: number }> {
  const counts = new Map<string, { user_id: string; username: string; count: number }>();
  for (const e of activityTimeline.value ?? []) {
    if (!VOCAL_EVENTS.includes(e.event_type)) continue;
    const m = e.metadata as Meta;
    const companions = m?.companions;
    if (Array.isArray(companions)) {
      for (const c of companions) {
        if (c && typeof c === "object") {
          const cid = (c as Record<string, unknown>).user_id as string | undefined;
          const cname = (c as Record<string, unknown>).username as string | undefined;
          if (!cid) continue;
          const cur = counts.get(cid) ?? { user_id: cid, username: cname ?? cid, count: 0 };
          cur.count++;
          counts.set(cid, cur);
        } else if (typeof c === "string") {
          const cur = counts.get(c) ?? { user_id: c, username: c, count: 0 };
          cur.count++;
          counts.set(c, cur);
        }
      }
    }
  }
  return [...counts.values()].sort((a, b) => b.count - a.count).slice(0, limit);
}

function heatmapData() {
  const days = ["Lun", "Mar", "Mer", "Jeu", "Ven", "Sam", "Dim"];
  const grid: number[][] = Array.from({ length: 7 }, () => Array(24).fill(0));
  let max = 0;
  for (const e of activityTimeline.value ?? []) {
    if (!TEXT_EVENTS.includes(e.event_type)) continue;
    const d = new Date(e.created_at);
    const dow = (d.getDay() + 6) % 7;
    const h = d.getHours();
    grid[dow][h]++;
    if (grid[dow][h] > max) max = grid[dow][h];
  }
  return { days, grid, max };
}

function heatColor(value: number, max: number): string {
  if (value === 0) return "rgba(88, 101, 242, 0.05)";
  const intensity = Math.min(value / Math.max(1, max), 1);
  return `rgba(88, 101, 242, ${0.1 + intensity * 0.8})`;
}

function burstCount(): number {
  const list = activityTimeline.value
    ?.filter((e) => e.event_type === "message_sent")
    .map((e) => new Date(e.created_at).getTime())
    .sort((a, b) => a - b) ?? [];
  if (list.length < 10) return 0;
  let bursts = 0;
  for (let i = 0; i + 9 < list.length; i++) {
    if (list[i + 9] - list[i] <= 60_000) {
      bursts++;
      i += 9;
    }
  }
  return bursts;
}

function automodCount(): number {
  if (!dossier.value) return 0;
  return dossier.value.security_events.filter(
    (e) => typeof e.event_type === "string" && e.event_type.toLowerCase().includes("automod"),
  ).length;
}

function watchSplitStats() {
  const dossierVal = dossier.value;
  if (!dossierVal) return null;
  const since = dossierVal.user.first_seen_at;
  if (!since) return null;
  const sinceTs = new Date(since).getTime();
  let beforeIncidents = 0, afterIncidents = 0;
  for (const inf of dossierVal.infractions) {
    const ts = new Date(inf.created_at).getTime();
    if (ts < sinceTs) beforeIncidents++;
    else afterIncidents++;
  }
  return { sinceTs, beforeIncidents, afterIncidents };
}

function discordProfileUrl(userId: string): string {
  return `https://discord.com/users/${userId}`;
}
</script>

<template>
  <div v-if="selectedMember" class="card card--lg detail-panel">
    <div class="panel-top-actions">
      <a
        :href="discordProfileUrl(selectedMember.member.user_id)"
        target="_blank"
        rel="noopener noreferrer"
        class="discord-link-btn"
        title="Ouvrir le profil Discord de l'utilisateur"
      >
        <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
          <path d="M20.317 4.37a19.791 19.791 0 00-4.885-1.515.074.074 0 00-.079.037c-.21.375-.444.864-.608 1.25a18.27 18.27 0 00-5.487 0 12.64 12.64 0 00-.617-1.25.077.077 0 00-.079-.037A19.736 19.736 0 003.677 4.37a.07.07 0 00-.032.027C.533 9.046-.32 13.58.099 18.057a.082.082 0 00.031.057 19.9 19.9 0 005.993 3.03.078.078 0 00.084-.028 14.09 14.09 0 001.226-1.994.076.076 0 00-.041-.106 13.107 13.107 0 01-1.872-.892.077.077 0 01-.008-.128 10.2 10.2 0 00.372-.292.074.074 0 01.077-.01c3.928 1.793 8.18 1.793 12.062 0a.074.074 0 01.078.01c.12.098.246.198.373.292a.077.077 0 01-.006.127 12.299 12.299 0 01-1.873.892.077.077 0 00-.041.107c.36.698.772 1.362 1.225 1.993a.076.076 0 00.084.028 19.839 19.839 0 006.002-3.03.077.077 0 00.032-.054c.5-5.177-.838-9.674-3.549-13.66a.061.061 0 00-.031-.03zM8.02 15.33c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.956-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.956 2.418-2.157 2.418zm7.975 0c-1.183 0-2.157-1.085-2.157-2.419 0-1.333.955-2.419 2.157-2.419 1.21 0 2.176 1.096 2.157 2.42 0 1.333-.946 2.418-2.157 2.418z"/>
        </svg>
        Voir profil Discord
      </a>
      <button
        v-if="!isWatched(selectedMember.member.user_id)"
        class="watch-top-btn"
        :disabled="watchAction"
        @click="toggleWatch"
      >
        + Surveiller
      </button>
      <button
        v-else
        class="unwatch-top-btn"
        :disabled="watchAction"
        @click="unwatch"
      >
        Retirer surveillance
      </button>
      <button
        class="reset-top-btn"
        :disabled="resetting"
        title="Supprimer toutes les donnees de moderation de ce membre (irreversible)"
        @click="handleReset"
      >
        {{ resetting ? "Nettoyage…" : "Tout reinitialiser" }}
      </button>
      <button class="close-btn" @click="closeMember">&times;</button>
    </div>

    <div v-if="loadingSummary" class="loading">Chargement...</div>
    <template v-else>
      <div class="profile-header">
        <div class="avatar-placeholder profile-avatar">{{ selectedMember.member.username.charAt(0).toUpperCase() }}</div>
        <div class="profile-info">
          <h2>{{ selectedMember.member.display_name || selectedMember.member.username }}</h2>
          <span class="profile-id">{{ selectedMember.member.user_id }}</span>
        </div>
      </div>

      <AppTabs
        :model-value="detailTab"
        :tabs="detailTabsItems"
        class="detail-tabs-wrap"
        @update:model-value="(k) => (detailTab = k as typeof detailTab)"
      />

      <!-- ── TAB: Profil ── -->
      <div v-if="detailTab === 'profil'" class="tab-content">
        <div class="profile-meta">
          <div class="meta-item">
            <span class="meta-label">Membre depuis</span>
            <span class="meta-value">{{ formatDate(selectedMember.member.joined_at) }}</span>
          </div>
          <div class="meta-item">
            <span class="meta-label">Compte cree</span>
            <span class="meta-value">{{ formatDate(selectedMember.member.account_created) }}</span>
          </div>
          <div class="meta-item">
            <span class="meta-label">Roles</span>
            <span class="meta-value">{{ rolesCount(selectedMember.member.roles) }}</span>
          </div>
        </div>

        <div class="stats-row">
          <div class="stat-box">
            <span class="stat-number">{{ selectedMember.stats.message_count }}</span>
            <span class="stat-text">Messages</span>
          </div>
          <div class="stat-box">
            <span class="stat-number">{{ formatDuration(selectedMember.stats.voice_seconds) }}</span>
            <span class="stat-text">Vocal</span>
          </div>
          <div class="stat-box">
            <span class="stat-number">{{ selectedMember.infractions.total }}</span>
            <span class="stat-text">Infractions</span>
          </div>
          <div class="stat-box">
            <span class="stat-number stat-warn">{{ selectedMember.moderation.total_warns }}</span>
            <span class="stat-text">Warns</span>
          </div>
          <div class="stat-box">
            <span class="stat-number stat-mute">{{ selectedMember.moderation.total_mutes }}</span>
            <span class="stat-text">Mutes</span>
          </div>
          <div class="stat-box">
            <span class="stat-number stat-ban">{{ selectedMember.moderation.total_bans }}</span>
            <span class="stat-text">Bans</span>
          </div>
        </div>

        <div v-if="selectedMember.infractions.recent.length > 0" class="section">
          <h3>Infractions recentes</h3>
          <div v-for="(inf, i) in selectedMember.infractions.recent" :key="i" class="detail-row">
            <div class="detail-row-header">
              <span class="detail-date">{{ formatDate(inf.created_at as string) }}</span>
              <AppBadge :label="String(inf.action)" variant="danger" />
            </div>
            <div class="detail-row-body">{{ inf.reason }}</div>
          </div>
        </div>
      </div>

      <!-- ── TAB: Surveillance ── -->
      <div v-if="detailTab === 'surveillance'" class="tab-content">
        <div v-if="dossierLoading" class="loading">Chargement...</div>
        <template v-else>
          <template v-if="isWatched(selectedMember.member.user_id) && dossier">
            <div class="dossier-summary">
              <div class="summary-card">
                <span class="summary-value">{{ dossier.user.risk_level }}</span>
                <span class="summary-label">Risque</span>
              </div>
              <div class="summary-card">
                <span class="summary-value">{{ dossier.user.total_warns + dossier.user.total_mutes + dossier.user.total_bans }}</span>
                <span class="summary-label">Sanctions</span>
              </div>
              <div class="summary-card">
                <span class="summary-value">{{ dossier.user.security_events_count }}</span>
                <span class="summary-label">Evt Securite</span>
              </div>
            </div>

            <div v-if="dossier.infractions.length > 0" class="section">
              <h3>Infractions ({{ dossier.infractions.length }})</h3>
              <div v-for="inf in dossier.infractions.slice(0, 15)" :key="inf.id" class="detail-row">
                <div class="detail-row-header">
                  <span class="detail-date">{{ fmt(inf.created_at) }}</span>
                  <AppBadge :label="inf.action || inf.infraction_type || '?'" variant="danger" />
                </div>
                <div class="detail-row-body">{{ inf.reason }}</div>
                <div v-if="inf.score" class="detail-row-sub">Score: {{ inf.score }}</div>
              </div>
            </div>

            <div v-if="dossier.moderation_actions.length > 0" class="section">
              <h3>Actions de moderation ({{ dossier.moderation_actions.length }})</h3>
              <div v-for="act in dossier.moderation_actions.slice(0, 15)" :key="act.id" class="detail-row">
                <div class="detail-row-header">
                  <span class="detail-date">{{ act.id.slice(0, 8) }}</span>
                  <AppBadge :label="act.action_type" variant="warning" />
                </div>
                <div class="detail-row-body">{{ act.reason }}</div>
                <div class="detail-row-sub">Cible: {{ act.target_name }}</div>
              </div>
            </div>

            <div v-if="dossier.security_events.length > 0" class="section">
              <h3>Evenements de securite ({{ dossier.security_events.length }})</h3>
              <div v-for="evt in dossier.security_events.slice(0, 10)" :key="evt.id" class="detail-row">
                <div class="detail-row-header">
                  <span class="detail-date">{{ fmt(evt.created_at) }}</span>
                  <AppBadge
                    :label="evt.severity"
                    :variant="evt.severity === 'critical' ? 'danger' : evt.severity === 'warning' ? 'warning' : 'info'"
                  />
                </div>
                <div class="detail-row-body">{{ evt.description }}</div>
              </div>
            </div>

            <div v-if="activityTimeline && activityTimeline.length > 0" class="section watch-summary">
              <h3>📊 Vue d'ensemble</h3>
              <div class="watch-stats-grid">
                <div class="watch-stat-card">
                  <span class="watch-stat-label">Messages</span>
                  <div class="watch-stat-multi">
                    <span><strong>{{ countByCategory(0, 'text') }}</strong> total</span>
                    <span class="muted">{{ countByCategory(30, 'text') }} · 30j</span>
                    <span class="muted">{{ countByCategory(7, 'text') }} · 7j</span>
                  </div>
                </div>
                <div class="watch-stat-card">
                  <span class="watch-stat-label">Heures vocales</span>
                  <div class="watch-stat-multi">
                    <span><strong>{{ voiceHours(0) }}h</strong> total</span>
                    <span class="muted">{{ voiceHours(30) }}h · 30j</span>
                    <span class="muted">{{ voiceHours(7) }}h · 7j</span>
                  </div>
                </div>
                <div class="watch-stat-card">
                  <span class="watch-stat-label">Pièces jointes</span>
                  <div class="watch-stat-multi">
                    <span>📷 <strong>{{ attachmentCounts().images }}</strong></span>
                    <span>🎬 <strong>{{ attachmentCounts().videos }}</strong></span>
                    <span>📎 <strong>{{ attachmentCounts().files }}</strong></span>
                    <span>🔗 <strong>{{ attachmentCounts().links }}</strong></span>
                  </div>
                </div>
                <div class="watch-stat-card">
                  <span class="watch-stat-label">Modération</span>
                  <div class="watch-stat-multi">
                    <span><strong>{{ dossier?.infractions.length ?? 0 }}</strong> infractions</span>
                    <span class="muted">🤖 {{ automodCount() }} automod</span>
                    <span class="muted">⚡ {{ burstCount() }} burst{{ burstCount() > 1 ? 's' : '' }} (10msg/60s)</span>
                  </div>
                </div>
                <div v-if="watchSplitStats()" class="watch-stat-card">
                  <span class="watch-stat-label">Sous surveillance depuis</span>
                  <div class="watch-stat-multi">
                    <span><strong>{{ formatDate(dossier?.user.first_seen_at as string ?? null) }}</strong></span>
                    <span class="muted">Avant : {{ watchSplitStats()?.beforeIncidents ?? 0 }} incident(s)</span>
                    <span class="muted">Depuis : {{ watchSplitStats()?.afterIncidents ?? 0 }} incident(s)</span>
                  </div>
                </div>
              </div>
            </div>

            <div v-if="heatmapData().max > 0" class="section">
              <h3>🗓️ Heatmap activité (messages par heure)</h3>
              <div class="heatmap-wrap">
                <table class="watch-heatmap">
                  <thead>
                    <tr>
                      <th></th>
                      <th v-for="h in 24" :key="h" class="hm-hour">{{ h - 1 }}h</th>
                    </tr>
                  </thead>
                  <tbody>
                    <tr v-for="(dn, di) in heatmapData().days" :key="di">
                      <td class="hm-day">{{ dn }}</td>
                      <td
                        v-for="hi in 24"
                        :key="hi"
                        class="hm-cell"
                        :style="{ backgroundColor: heatColor(heatmapData().grid[di][hi - 1], heatmapData().max) }"
                        :title="`${dn} ${hi - 1}h : ${heatmapData().grid[di][hi - 1]} msg`"
                      ></td>
                    </tr>
                  </tbody>
                </table>
              </div>
            </div>

            <div v-if="topChannels().length > 0 || topVoiceCompanions().length > 0" class="section watch-tops">
              <div v-if="topChannels().length > 0" class="watch-tops-col">
                <h3>🏆 Top salons</h3>
                <ul class="watch-rank">
                  <li v-for="(c, i) in topChannels()" :key="c.id">
                    <span class="rank-pos">#{{ i + 1 }}</span>
                    <span class="rank-name">#{{ c.name }}</span>
                    <span class="rank-count">{{ c.count }} msg</span>
                  </li>
                </ul>
              </div>
              <div v-if="topVoiceCompanions().length > 0" class="watch-tops-col">
                <h3>👥 Compagnons vocaux</h3>
                <ul class="watch-rank">
                  <li v-for="(c, i) in topVoiceCompanions()" :key="c.user_id">
                    <span class="rank-pos">#{{ i + 1 }}</span>
                    <span class="rank-name">{{ c.username }}</span>
                    <span class="rank-count">{{ c.count }}×</span>
                  </li>
                </ul>
              </div>
            </div>

            <div v-if="activityTimeline && activityTimeline.length > 0" class="section">
              <h3>Activite recente ({{ filteredActivity.length }} / {{ activityTimeline.length }})</h3>

              <div class="activity-stats">
                <span><strong>{{ activityCount('message_sent') }}</strong> messages</span>
                <span><strong>{{ activityCount('voice_join') }}</strong> entrees vocal</span>
                <span><strong>{{ activityCount('voice_leave') }}</strong> sorties vocal</span>
                <span><strong>{{ activityCount('voice_move') }}</strong> moves</span>
                <span><strong>{{ activityCount('message_deleted') }}</strong> supprimes</span>
                <span><strong>{{ activityCount('message_edited') }}</strong> edites</span>
                <span><strong>{{ activityLinksCount() }}</strong> liens</span>
                <span><strong>{{ activityAttachmentsCount() }}</strong> pieces jointes</span>
              </div>

              <div class="activity-filters">
                <div class="activity-filter-group">
                  <button
                    v-for="t in (['all', 'text', 'vocal', 'other'] as const)"
                    :key="t"
                    type="button"
                    :class="['activity-chip', { active: activityTypeFilter === t }]"
                    @click="activityTypeFilter = t"
                  >
                    {{ t === 'all' ? 'Tout' : t === 'text' ? 'Texte' : t === 'vocal' ? 'Vocal' : 'Autre' }}
                  </button>
                </div>
                <div class="activity-filter-group">
                  <label class="activity-date-label">
                    Du
                    <input v-model="activityDateFrom" type="date" class="activity-date-input" />
                  </label>
                  <label class="activity-date-label">
                    Au
                    <input v-model="activityDateTo" type="date" class="activity-date-input" />
                  </label>
                  <button
                    v-if="activityTypeFilter !== 'all' || activityDateFrom || activityDateTo"
                    type="button"
                    class="activity-reset"
                    @click="resetActivityFilters"
                  >
                    Reset
                  </button>
                </div>
              </div>

              <div v-if="filteredActivity.length === 0" class="empty-small">
                Aucun evenement ne correspond aux filtres.
              </div>
              <div v-for="evt in activityPageRows" :key="evt.id" class="detail-row">
                <div class="detail-row-header">
                  <span class="detail-date">{{ fmt(evt.created_at) }}</span>
                  <div class="header-badges">
                    <AppBadge :label="activityLabel(evt.event_type)" :variant="activityVariant(evt.event_type)" />
                    <span v-if="evt.event_type.startsWith('voice_')" class="activity-channel">
                      {{ voiceChannelLabel(evt) }}
                    </span>
                    <span v-else-if="evt.channel_name" class="activity-channel">
                      #{{ evt.channel_name }}<span v-if="evt.channel_id" class="channel-id">({{ evt.channel_id }})</span>
                    </span>
                    <span v-else-if="evt.channel_id" class="activity-channel">
                      #{{ evt.channel_id }}
                    </span>
                  </div>
                </div>

                <template v-if="evt.event_type === 'message_edited'">
                  <div class="diff-block">
                    <div class="diff-row diff-before">
                      <span class="diff-label">Avant :</span>
                      <span v-if="editedBeforeAfter(evt).before" class="diff-content">{{ editedBeforeAfter(evt).before }}</span>
                      <span v-else class="diff-content diff-missing">
                        <em>(message non disponible — n'était pas dans le cache du bot au moment de la modif)</em>
                      </span>
                    </div>
                    <div class="diff-row diff-after">
                      <span class="diff-label">Après :</span>
                      <span v-if="editedBeforeAfter(evt).after" class="diff-content">{{ editedBeforeAfter(evt).after }}</span>
                      <span v-else class="diff-content diff-missing">
                        <em>(contenu vide)</em>
                      </span>
                    </div>
                  </div>
                </template>

                <template v-else-if="evt.event_type === 'roles_changed'">
                  <div class="diff-block">
                    <div v-if="rolesDiff(evt).added.length > 0" class="diff-row diff-after">
                      <span class="diff-label">+ Ajoutés :</span>
                      <span class="diff-content">{{ rolesDiff(evt).added.join(', ') }}</span>
                    </div>
                    <div v-if="rolesDiff(evt).removed.length > 0" class="diff-row diff-before">
                      <span class="diff-label">− Retirés :</span>
                      <span class="diff-content">{{ rolesDiff(evt).removed.join(', ') }}</span>
                    </div>
                    <div
                      v-if="rolesDiff(evt).added.length === 0 && rolesDiff(evt).removed.length === 0 && evt.content"
                      class="detail-row-body"
                    >{{ evt.content }}</div>
                  </div>
                </template>

                <template v-else-if="evt.event_type === 'nickname_changed'">
                  <div class="diff-block">
                    <div v-if="profileDiff(evt).before" class="diff-row diff-before">
                      <span class="diff-label">Ancien pseudo :</span>
                      <span class="diff-content">{{ profileDiff(evt).before }}</span>
                    </div>
                    <div v-if="profileDiff(evt).after" class="diff-row diff-after">
                      <span class="diff-label">Nouveau :</span>
                      <span class="diff-content">{{ profileDiff(evt).after }}</span>
                    </div>
                  </div>
                </template>

                <template v-else-if="evt.event_type === 'avatar_changed'">
                  <div class="avatar-diff">
                    <div v-if="safeImageUrl(avatarDiff(evt).before)" class="avatar-cell">
                      <span class="diff-label">Avant</span>
                      <img :src="safeImageUrl(avatarDiff(evt).before) ?? ''" alt="ancien avatar" class="avatar-thumb" />
                    </div>
                    <span v-if="safeImageUrl(avatarDiff(evt).before) && safeImageUrl(avatarDiff(evt).after)" class="diff-arrow">→</span>
                    <div v-if="safeImageUrl(avatarDiff(evt).after)" class="avatar-cell">
                      <span class="diff-label">Après</span>
                      <img :src="safeImageUrl(avatarDiff(evt).after) ?? ''" alt="nouvel avatar" class="avatar-thumb" />
                    </div>
                    <span v-if="!avatarDiff(evt).before && !avatarDiff(evt).after && evt.content" class="detail-row-body">{{ evt.content }}</span>
                  </div>
                </template>

                <template v-else>
                  <div v-if="evt.content" class="detail-row-body">{{ evt.content }}</div>
                </template>
              </div>

              <PaginationBar
                v-if="filteredActivity.length > 0"
                :current-page="activityPage"
                :total-pages="activityTotalPages"
                :total-items="filteredActivity.length"
                :per-page="activityPerPage"
                @update:current-page="activityPage = $event"
                @update:per-page="(n) => { activityPerPage = n; activityPage = 1; }"
              />
            </div>

            <div v-if="dossier.notes && dossier.notes.length > 0" class="section">
              <h3>Notes ({{ dossier.notes.length }})</h3>
              <div v-for="(note, i) in dossier.notes" :key="i" class="detail-row">
                <div class="detail-row-header">
                  <span class="detail-date">{{ note.created_at ? fmt(String(note.created_at)) : '' }}</span>
                  <span class="note-author">{{ note.author_name }}</span>
                </div>
                <div class="detail-row-body">{{ note.content }}</div>
              </div>
            </div>
          </template>

          <div v-else class="empty-small">
            Ce membre n'est pas sous surveillance. Cliquez sur le bouton ci-dessus pour l'ajouter.
          </div>
        </template>
      </div>
    </template>
  </div>
</template>

<style scoped>
.detail-panel {
  flex: 1;
  overflow-y: auto;
  max-height: calc(100vh - 240px);
  position: relative;
}

.panel-top-actions {
  position: absolute;
  top: 16px;
  right: 16px;
  display: flex;
  align-items: center;
  gap: 8px;
}

.unwatch-top-btn {
  padding: 6px 14px;
  border: 1px solid var(--danger);
  border-radius: 8px;
  background: var(--danger-bg);
  color: var(--danger);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}
.unwatch-top-btn:hover:not(:disabled) { background: var(--danger); color: white; }
.unwatch-top-btn:disabled { opacity: 0.4; cursor: not-allowed; }

.watch-top-btn {
  padding: 6px 14px;
  border: 1px solid var(--warning);
  border-radius: 8px;
  background: var(--warning-bg);
  color: var(--warning);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}
.watch-top-btn:hover:not(:disabled) { background: var(--warning); color: #0a0a0a; }
.watch-top-btn:disabled { opacity: 0.4; cursor: not-allowed; }

.reset-top-btn {
  padding: 6px 14px;
  border: 1px solid var(--danger);
  border-radius: 8px;
  background: transparent;
  color: var(--danger);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}
.reset-top-btn:hover:not(:disabled) { background: var(--danger); color: white; }
.reset-top-btn:disabled { opacity: 0.4; cursor: not-allowed; }

.close-btn {
  background: none;
  border: 1px solid var(--border);
  color: var(--text-secondary);
  width: 32px;
  height: 32px;
  border-radius: 8px;
  font-size: 18px;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.close-btn:hover { background: var(--bg-hover); color: var(--text-primary); }

.profile-header {
  display: flex;
  align-items: center;
  gap: 16px;
  margin-bottom: 16px;
}

.profile-avatar { width: 56px; height: 56px; font-size: 24px; }
.profile-info h2 { margin: 0; font-size: 20px; }
.profile-id {
  font-size: 12px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.discord-link-btn {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 12px;
  border-radius: 8px;
  background: #5865F2;
  color: white;
  text-decoration: none;
  font-size: 12px;
  font-weight: 600;
  transition: background 0.2s ease, transform 0.2s ease;
  white-space: nowrap;
}
.discord-link-btn:hover { background: #4752c4; transform: translateY(-1px); }

.detail-tabs-wrap { margin-bottom: 16px; }
.tab-content { margin-top: 4px; }
.loading { color: var(--text-secondary); padding: 40px; text-align: center; }
.empty-small { color: var(--text-secondary); text-align: center; padding: 20px; font-size: 13px; }

/* Profil tab */
.profile-meta {
  display: flex;
  gap: 24px;
  margin-bottom: 20px;
  padding: 12px 16px;
  background: var(--bg-secondary);
  border-radius: 8px;
}
.meta-item { display: flex; flex-direction: column; gap: 2px; }
.meta-label { font-size: 11px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.3px; }
.meta-value { font-size: 13px; font-weight: 600; color: var(--text-primary); }

.stats-row { display: flex; gap: 10px; margin-bottom: 20px; }
.stat-box {
  flex: 1;
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 12px 8px;
  text-align: center;
}
.stat-number { display: block; font-size: 18px; font-weight: 700; color: var(--text-primary); }
.stat-warn { color: var(--info) !important; }
.stat-mute { color: var(--warning) !important; }
.stat-ban { color: var(--danger) !important; }
.stat-text { font-size: 10px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.3px; }

.section { margin-bottom: 20px; }
.section h3 { margin: 0 0 10px 0; font-size: 14px; font-weight: 600; }

.detail-row {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 10px 14px;
  margin-bottom: 6px;
}
.detail-row-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}
.header-badges {
  display: flex;
  align-items: center;
  gap: 6px;
  margin-left: auto;
  flex-wrap: wrap;
  justify-content: flex-end;
}
.detail-date {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  flex-shrink: 0;
}
.detail-row-body { font-size: 13px; color: var(--text-primary); white-space: pre-wrap; word-break: break-word; }
.detail-row-sub { font-size: 11px; color: var(--text-secondary); margin-top: 4px; }

.diff-block { display: flex; flex-direction: column; gap: 4px; font-size: 13px; margin-top: 4px; }
.diff-row {
  display: flex;
  align-items: flex-start;
  gap: 8px;
  padding: 6px 10px;
  border-radius: 6px;
  border-left: 3px solid transparent;
}
.diff-before {
  background: color-mix(in srgb, var(--danger) 8%, transparent);
  border-left-color: color-mix(in srgb, var(--danger) 70%, transparent);
}
.diff-after {
  background: color-mix(in srgb, var(--success) 8%, transparent);
  border-left-color: color-mix(in srgb, var(--success) 70%, transparent);
}
.diff-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
  flex-shrink: 0;
  min-width: 90px;
}
.diff-content { color: var(--text-primary); white-space: pre-wrap; word-break: break-word; }
.diff-missing { color: var(--text-secondary); font-style: italic; }

.avatar-diff { display: flex; align-items: center; gap: 12px; margin-top: 6px; }
.avatar-cell { display: flex; flex-direction: column; align-items: center; gap: 4px; }
.avatar-thumb {
  width: 56px;
  height: 56px;
  border-radius: 50%;
  border: 1px solid var(--border);
  object-fit: cover;
}
.diff-arrow { font-size: 20px; color: var(--text-secondary); }

.channel-id { margin-left: 4px; font-size: 10px; opacity: 0.7; }

/* Surveillance tab */
.dossier-summary { display: flex; gap: 12px; margin-bottom: 20px; }
.summary-card {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 12px 16px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 100px;
}
.summary-value { font-weight: 700; font-size: 16px; color: var(--text-primary); text-transform: capitalize; }
.summary-label { font-size: 10px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.3px; }

.note-author { font-size: 12px; font-weight: 600; color: var(--accent); }

.activity-stats {
  display: flex;
  flex-wrap: wrap;
  gap: 8px 16px;
  padding: 10px 12px;
  margin-bottom: 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  font-size: 12px;
  color: var(--text-secondary);
}
.activity-stats strong { color: var(--text-primary); font-weight: 700; margin-right: 2px; }
.activity-channel {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", monospace;
  background: var(--bg-hover);
  padding: 1px 6px;
  border-radius: var(--radius-sm);
}

/* Surveillance enrichie */
.watch-summary { margin-bottom: 16px; }
.watch-stats-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
  gap: 12px;
  margin-top: 8px;
}
.watch-stat-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 14px;
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.watch-stat-label {
  font-size: 11px;
  font-weight: 700;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.4px;
}
.watch-stat-multi { display: flex; flex-direction: column; gap: 2px; font-size: 13px; }
.watch-stat-multi .muted { color: var(--text-secondary); font-size: 11px; }

.heatmap-wrap { width: 100%; overflow-x: auto; }
.watch-heatmap {
  border-collapse: separate;
  border-spacing: 2px;
  width: 100%;
  min-width: 540px;
  table-layout: fixed;
}
.hm-hour { font-size: 9px; color: var(--text-secondary); padding: 1px 0; text-align: center; }
.hm-day {
  font-size: 11px;
  color: var(--text-secondary);
  padding-right: 6px;
  white-space: nowrap;
  text-align: right;
  width: 36px;
}
.hm-cell { height: 22px; border-radius: 3px; cursor: default; }

.watch-tops { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
@media (max-width: 700px) {
  .watch-tops { grid-template-columns: 1fr; }
}
.watch-tops-col {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 10px 14px;
}
.watch-tops-col h3 { margin: 0 0 8px; font-size: 13px; }
.watch-rank { list-style: none; margin: 0; padding: 0; }
.watch-rank li {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 13px;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 50%, transparent);
}
.watch-rank li:last-child { border-bottom: none; }
.rank-pos { font-weight: 700; color: var(--accent); min-width: 28px; }
.rank-name { flex: 1; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.rank-count { color: var(--text-secondary); font-size: 12px; }

.activity-filters { display: flex; flex-wrap: wrap; gap: 12px; align-items: center; margin-bottom: 12px; }
.activity-filter-group { display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }

.activity-chip {
  padding: 5px 12px;
  border-radius: var(--radius-sm);
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-secondary);
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}
.activity-chip:hover { color: var(--text-primary); background: var(--bg-hover); }
.activity-chip.active { background: var(--accent); border-color: var(--accent); color: white; }

.activity-date-label {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  font-size: 11px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.6px;
}
.activity-date-input {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 6px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
}
.activity-date-input:focus { outline: none; border-color: var(--accent); }

.activity-reset {
  padding: 4px 10px;
  border: 1px solid var(--border);
  border-radius: var(--radius-sm);
  background: transparent;
  color: var(--text-secondary);
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
}
.activity-reset:hover { color: var(--danger); border-color: var(--danger); }
</style>
