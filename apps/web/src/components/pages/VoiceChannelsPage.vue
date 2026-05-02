<script setup lang="ts">
import { ref, computed } from "vue";
import { useVoiceChannels, useVoiceChannelDetail } from "../../composables/useVoiceChannels";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";
import { usePagination } from "../../composables/usePagination";
import { useConfirm } from "../../composables/useConfirm";
import AppBadge from "../atoms/AppBadge.vue";
import ErrorState from "../atoms/ErrorState.vue";
import PaginationBar from "../molecules/PaginationBar.vue";
import { useFormatDate } from "../../composables/useFormatDate";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

const { visible } = useComponentVisibility();

const { formatShortDateTime: fmt } = useFormatDate();
const { confirm: confirmDialog } = useConfirm();

const {
  filteredChannels,
  loading,
  error,
  filterKind,
  publicCount,
  privateCount,
  totalCount,
  closing,
  cleaningAll,
  historyChannels,
  historyLoading,
  fetchChannels,
  fetchHistory,
  closeChannel,
  closeAllDisplayed,
  purging,
  purgeChannel,
  purgingAll,
  purgeAllHistory,
} = useVoiceChannels();

async function handlePurgeAll() {
  const ok = await confirmDialog({
    title: "Vider l'historique",
    message:
      `Supprimer definitivement les ${historyChannels.value.length} salon(s) de l'historique ?\n\n` +
      "Toutes les lignes fermees et leurs timelines seront effacees de la BDD. Action irreversible.",
  });
  if (!ok) return;
  await purgeAllHistory();
}

async function handlePurge(channelId: string, channelName: string) {
  const ok = await confirmDialog({
    title: "Supprimer de l'historique",
    message: `Supprimer definitivement "${channelName}" ?\n\nLa ligne et sa timeline seront effacees de la BDD. Cette action est irreversible.`,
  });
  if (!ok) return;
  await purgeChannel(channelId);
}
useRealtimeRefresh(
  ["voice_channel_created", "voice_channel_closed", "voice_channel_updated", "voice_invite_created", "voice_invite_used", "voice_invite_revoked"],
  async () => {
    await Promise.all([fetchChannels(), fetchHistory()]);
  },
);
const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedChannels } = usePagination(filteredChannels);

const historyFilterKind = ref<"all" | "public" | "private">("all");
const historySearch = ref("");
const historyFrom = ref("");
const historyTo = ref("");

const filteredHistory = computed(() => {
  const q = historySearch.value.trim().toLowerCase();
  const from = historyFrom.value ? new Date(historyFrom.value).getTime() : null;
  const to = historyTo.value ? new Date(historyTo.value).getTime() + 86_400_000 : null;
  return historyChannels.value.filter((c) => {
    if (historyFilterKind.value !== "all" && c.kind !== historyFilterKind.value) return false;
    if (q && !c.channel_name.toLowerCase().includes(q) && !c.owner_name.toLowerCase().includes(q)) return false;
    const created = new Date(c.created_at).getTime();
    if (from !== null && created < from) return false;
    if (to !== null && created >= to) return false;
    return true;
  });
});

function resetHistoryFilters() {
  historyFilterKind.value = "all";
  historySearch.value = "";
  historyFrom.value = "";
  historyTo.value = "";
}

const {
  currentPage: historyPage,
  perPage: historyPerPage,
  totalItems: historyTotal,
  totalPages: historyTotalPages,
  paginatedItems: paginatedHistory,
} = usePagination(filteredHistory);

const { detail, events, loading: detailLoading, eventsLoading, fetchDetail } = useVoiceChannelDetail();

function eventLabel(type: string): string {
  switch (type) {
    case "voice_join": return "Entree";
    case "voice_leave": return "Sortie";
    case "voice_move": return "Deplacement";
    case "voice_channel_created": return "Salon cree";
    case "voice_channel_updated": return "Modification";
    case "voice_channel_closed": return "Salon ferme";
    default: return type;
  }
}

function eventVariant(type: string): "info" | "warning" | "danger" | "default" {
  switch (type) {
    case "voice_join":
    case "voice_channel_created": return "info";
    case "voice_leave":
    case "voice_move": return "default";
    case "voice_channel_updated": return "warning";
    case "voice_channel_closed": return "danger";
    default: return "default";
  }
}

function formatEventDetails(type: string, details: Record<string, unknown>): string {
  if (!details || Object.keys(details).length === 0) return "";
  if (type === "voice_move") {
    const from = details.from_channel ?? "?";
    const to = details.to_channel ?? "?";
    return `${from} -> ${to}`;
  }
  const entries = Object.entries(details).filter(([, v]) => v !== null && v !== undefined);
  if (entries.length === 0) return "";
  return entries.map(([k, v]) => `${k}: ${JSON.stringify(v)}`).join(", ");
}

async function handleCloseChannel(channelId: string, channelName: string) {
  const ok = await confirmDialog({
    title: "Fermer le salon",
    message: `Fermer "${channelName}" ?\n\nLa ligne sera marquee comme fermee en BDD et ne s'affichera plus ici. Aucune action cote Discord.`,
  });
  if (!ok) return;
  await closeChannel(channelId);
}

async function handleCleanupAll() {
  const ok = await confirmDialog({
    title: "Nettoyer tous les salons affiches",
    message:
      `Fermer ${filteredChannels.value.length} salon(s) actuellement affiches ?\n\n` +
      "Utile pour nettoyer les lignes fantomes laissees par un bot qui a crash " +
      "ou redemarre. Aucune action cote Discord, uniquement en BDD.",
  });
  if (!ok) return;
  await closeAllDisplayed();
}

const selectedId = ref<string | null>(null);

async function selectChannel(channelId: string) {
  selectedId.value = channelId;
  await fetchDetail(channelId);
}

function backToList() {
  selectedId.value = null;
  detail.value = null;
}

function kindVariant(kind: string): "info" | "warning" | "default" {
  switch (kind) {
    case "public": return "info";
    case "private": return "warning";
    default: return "default";
  }
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>Salons vocaux</h1>
      <p class="page-subtitle">Salons vocaux temporaires actifs</p>
    </header>

    <!-- Stats -->
    <div class="stats-row">
      <div class="stat-card">
        <span class="stat-value">{{ totalCount }}</span>
        <span class="stat-label">Total</span>
      </div>
      <div class="stat-card">
        <span class="stat-value">{{ publicCount }}</span>
        <span class="stat-label">Public</span>
      </div>
      <div class="stat-card">
        <span class="stat-value">{{ privateCount }}</span>
        <span class="stat-label">Prive</span>
      </div>
    </div>

    <!-- Detail view -->
    <div v-if="selectedId && detail" class="detail-view">
      <button class="back-btn" @click="backToList">&larr; Retour</button>

      <div v-if="detailLoading" class="loading">Chargement...</div>
      <div v-else class="detail-content">
        <h2>{{ detail.channel.channel_name }}</h2>
        <div class="detail-grid">
          <div><strong>Proprietaire :</strong> {{ detail.channel.owner_name }}</div>
          <div><strong>Type :</strong> <AppBadge :label="detail.channel.kind" :variant="kindVariant(detail.channel.kind)" /></div>
          <div><strong>Visibilite :</strong> {{ detail.channel.visibility }}</div>
          <div><strong>Verrouille :</strong> {{ detail.channel.locked ? 'Oui' : 'Non' }}</div>
          <div><strong>File d'attente :</strong> {{ detail.channel.queue_enabled ? 'Active' : 'Desactive' }}</div>
          <div v-if="detail.channel.member_limit"><strong>Limite :</strong> {{ detail.channel.member_limit }}</div>
          <div v-if="detail.channel.status"><strong>Statut :</strong> {{ detail.channel.status }}</div>
          <div><strong>Cree le :</strong> {{ fmt(detail.channel.created_at) }}</div>
        </div>

        <h3 v-if="detail.co_admins.length">Co-admins ({{ detail.co_admins.length }})</h3>
        <ul v-if="detail.co_admins.length" class="admin-list">
          <li v-for="ca in detail.co_admins" :key="ca.id">{{ ca.user_name }}</li>
        </ul>

        <h3>Timeline</h3>
        <div v-if="eventsLoading" class="loading">Chargement de la timeline...</div>
        <div v-else-if="events.length === 0" class="empty">Aucun evenement enregistre pour ce salon</div>
        <ul v-else class="timeline">
          <li v-for="ev in events" :key="ev.id" class="timeline-item">
            <span class="timeline-time">{{ fmt(ev.created_at) }}</span>
            <AppBadge :label="eventLabel(ev.event_type)" :variant="eventVariant(ev.event_type)" />
            <span v-if="ev.actor_name" class="timeline-actor">{{ ev.actor_name }}</span>
            <span class="timeline-details">{{ formatEventDetails(ev.event_type, ev.details) }}</span>
          </li>
        </ul>

        <h3 v-if="detail.bans.length">Bans ({{ detail.bans.length }})</h3>
        <div v-if="detail.bans.length" class="bans-table">
          <div v-for="ban in detail.bans" :key="ban.id" class="ban-row">
            <span>{{ ban.user_name }}</span>
            <span>par {{ ban.banned_by }}</span>
            <span v-if="ban.reason">{{ ban.reason }}</span>
            <span v-if="ban.expires_at">Expire : {{ new Date(ban.expires_at).toLocaleString() }}</span>
            <AppBadge v-else label="Permanent" variant="danger" />
          </div>
        </div>
      </div>
    </div>

    <!-- List view -->
    <div v-else>
      <div class="filter-row">
        <select v-model="filterKind" class="filter-select">
          <option value="all">Tous les types</option>
          <option value="public">Public</option>
          <option value="private">Prive</option>
        </select>
        <button
          v-if="filteredChannels.length > 0"
          class="cleanup-btn"
          :disabled="cleaningAll"
          title="Ferme tous les salons affiches en BDD (nettoyage des fantomes)"
          @click="handleCleanupAll"
        >
          {{ cleaningAll ? "Nettoyage…" : `Nettoyer tout (${filteredChannels.length})` }}
        </button>
      </div>

      <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchChannels" />
      <div v-else-if="loading" class="loading">Chargement...</div>
      <div v-else-if="filteredChannels.length === 0" class="empty">Aucun salon vocal temporaire actif</div>
      <table v-else class="data-table">
        <thead>
          <tr>
            <th>Nom</th>
            <th>Proprietaire</th>
            <th>Type</th>
            <th>Visibilite</th>
            <th>Verrouille</th>
            <th>File d'attente</th>
            <th>Creation</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="ch in paginatedChannels" :key="ch.id" class="clickable" @click="selectChannel(ch.channel_id)">
            <td>{{ ch.channel_name }}</td>
            <td>{{ ch.owner_name }}</td>
            <td><AppBadge :label="ch.kind" :variant="kindVariant(ch.kind)" /></td>
            <td>{{ ch.visibility }}</td>
            <td>{{ ch.locked ? 'Oui' : 'Non' }}</td>
            <td>{{ ch.queue_enabled ? 'Oui' : 'Non' }}</td>
            <td>{{ fmt(ch.created_at) }}</td>
            <td @click.stop>
              <button
                class="close-row-btn"
                :disabled="closing === ch.channel_id"
                title="Fermer ce salon en BDD"
                @click="handleCloseChannel(ch.channel_id, ch.channel_name)"
              >
                {{ closing === ch.channel_id ? "…" : "Fermer" }}
              </button>
            </td>
          </tr>
        </tbody>
      </table>

      <PaginationBar
        :current-page="currentPage"
        :total-pages="totalPages"
        :total-items="totalItems"
        :per-page="perPage"
        @update:current-page="currentPage = $event"
        @update:per-page="perPage = $event"
      />

      <!-- ======================================== -->
      <!-- Historique (salons fermes)              -->
      <!-- ======================================== -->
      <section class="history-section">
        <div class="history-head">
          <div>
            <h2 class="history-title">
              Historique
              <span class="history-count">{{ historyChannels.length }}</span>
            </h2>
            <p class="history-subtitle">Salons vocaux fermes / archives</p>
          </div>
          <button
            v-if="historyChannels.length > 0 && visible('db.purge.voice_history')"
            class="cleanup-btn"
            :disabled="purgingAll"
            title="Supprime definitivement tout l'historique en BDD (owner uniquement)"
            @click="handlePurgeAll"
          >
            {{ purgingAll ? "Suppression…" : `Tout supprimer (${historyChannels.length})` }}
          </button>
        </div>

        <div class="filter-row history-filters">
          <select v-model="historyFilterKind" class="filter-select">
            <option value="all">Tous les types</option>
            <option value="public">Public</option>
            <option value="private">Prive</option>
          </select>
          <input
            v-model="historySearch"
            type="search"
            placeholder="Rechercher nom ou proprietaire…"
            class="filter-input"
          />
          <input v-model="historyFrom" type="date" class="filter-input" title="Date de debut" />
          <input v-model="historyTo" type="date" class="filter-input" title="Date de fin" />
          <button class="reset-btn" type="button" @click="resetHistoryFilters">Reinitialiser</button>
        </div>

        <div v-if="historyLoading" class="loading">Chargement de l'historique...</div>
        <div v-else-if="historyChannels.length === 0" class="empty">
          Aucun salon dans l'historique
        </div>
        <div v-else-if="filteredHistory.length === 0" class="empty">
          Aucun salon ne correspond aux filtres
        </div>
        <table v-else class="data-table history-table">
          <thead>
            <tr>
              <th>Nom</th>
              <th>Proprietaire</th>
              <th>Type</th>
              <th>Creation</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr
              v-for="ch in paginatedHistory"
              :key="ch.id"
              class="clickable"
              @click="selectChannel(ch.channel_id)"
            >
              <td>{{ ch.channel_name }}</td>
              <td>{{ ch.owner_name }}</td>
              <td><AppBadge :label="ch.kind" :variant="kindVariant(ch.kind)" /></td>
              <td>{{ fmt(ch.created_at) }}</td>
              <td @click.stop>
                <button
                  v-if="visible('db.purge.voice_channel')"
                  class="close-row-btn"
                  :disabled="purging === ch.channel_id"
                  title="Supprimer definitivement cette ligne (owner uniquement)"
                  @click="handlePurge(ch.channel_id, ch.channel_name)"
                >
                  {{ purging === ch.channel_id ? "…" : "Supprimer" }}
                </button>
              </td>
            </tr>
          </tbody>
        </table>
        <PaginationBar
          v-if="filteredHistory.length > 0"
          :current-page="historyPage"
          :total-pages="historyTotalPages"
          :total-items="historyTotal"
          :per-page="historyPerPage"
          @update:current-page="historyPage = $event"
          @update:per-page="historyPerPage = $event"
        />
      </section>
    </div>
  </div>
</template>

<style scoped>
.page {
  padding: 24px;
}

.page-header {
  margin-bottom: 24px;
}

.page-header h1 {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-primary);
}

.page-subtitle {
  color: var(--text-secondary);
  font-size: 14px;
  margin-top: 4px;
}

.stats-row {
  display: flex;
  gap: 16px;
  margin-bottom: 24px;
}

.stat-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px 24px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 120px;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-primary);
}

.stat-label {
  font-size: 12px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.cleanup-btn {
  margin-left: auto;
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 7px 14px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.cleanup-btn:hover:not(:disabled) {
  background: var(--danger);
  color: white;
}

.cleanup-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.close-row-btn {
  background: transparent;
  color: var(--danger);
  border: 1px solid var(--danger);
  border-radius: 6px;
  padding: 4px 10px;
  font-size: 11px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
  white-space: nowrap;
}

.close-row-btn:hover:not(:disabled) {
  background: var(--danger);
  color: white;
}

.close-row-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.filter-row {
  margin-bottom: 16px;
  display: flex;
  align-items: center;
  gap: 12px;
  flex-wrap: wrap;
}

.filter-select,
.filter-input {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 13px;
}

.filter-input {
  min-width: 180px;
}

.reset-btn {
  background: transparent;
  color: var(--text-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 7px 14px;
  font-size: 12px;
  cursor: pointer;
}

.reset-btn:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.history-filters {
  margin-top: 4px;
  margin-bottom: 16px;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
}

.data-table th,
.data-table td {
  padding: 10px 14px;
  text-align: left;
  border-bottom: 1px solid var(--border);
  font-size: 13px;
}

.data-table th {
  color: var(--text-secondary);
  font-weight: 600;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.clickable {
  cursor: pointer;
}

.clickable:hover {
  background: var(--bg-hover);
}

.loading,
.empty {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary);
}

.back-btn {
  background: none;
  color: var(--accent);
  font-size: 14px;
  margin-bottom: 16px;
  cursor: pointer;
}

.back-btn:hover {
  text-decoration: underline;
}

.detail-content h2 {
  font-size: 20px;
  margin-bottom: 16px;
}

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px 24px;
  margin-bottom: 24px;
}

.detail-content h3 {
  font-size: 16px;
  margin-bottom: 8px;
  margin-top: 16px;
}

.admin-list {
  list-style: disc;
  padding-left: 20px;
}

.admin-list li {
  padding: 4px 0;
}

.ban-row {
  display: flex;
  gap: 16px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
  align-items: center;
  font-size: 13px;
}

.history-section {
  margin-top: 40px;
  padding-top: 24px;
  border-top: 1px solid var(--border);
}

.history-head {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 16px;
  margin-bottom: 16px;
}

.history-title {
  display: flex;
  align-items: center;
  gap: 10px;
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
  margin: 0 0 4px;
}

.history-count {
  font-size: 12px;
  font-weight: 600;
  padding: 2px 10px;
  border-radius: 999px;
  background-color: var(--bg-hover);
  color: var(--text-secondary);
}

.history-subtitle {
  color: var(--text-secondary);
  font-size: 12px;
  margin: 0 0 16px;
}

.history-table {
  opacity: 0.85;
}

.history-table tbody tr:hover {
  opacity: 1;
}

.timeline {
  list-style: none;
  padding: 0;
  margin: 0 0 24px;
  border-left: 2px solid var(--border);
}

.timeline-item {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  font-size: 13px;
  border-bottom: 1px solid var(--border);
}

.timeline-item:last-child {
  border-bottom: none;
}

.timeline-time {
  color: var(--text-secondary);
  font-size: 11px;
  min-width: 130px;
  font-variant-numeric: tabular-nums;
}

.timeline-actor {
  font-weight: 600;
  color: var(--text-primary);
}

.timeline-details {
  color: var(--text-secondary);
  font-size: 12px;
  font-family: monospace;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

/* Wrappers pour scroll horizontal sur tous les tableaux Voice */
:deep(.table-wrap),
.history-table-wrap {
  width: 100%;
  overflow-x: auto;
  -webkit-overflow-scrolling: touch;
}

@media (max-width: 768px) {
  /* KPI cards : grille auto-fit pour s'adapter */
  .stats-row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(120px, 1fr));
    gap: 8px;
  }
  .stat-card {
    min-width: 0;
    padding: 10px 14px;
  }
  .stat-value {
    font-size: 22px;
  }
  /* Tables debordent : on encapsule en overflow-x via :deep() le PaginationBar */
  table {
    font-size: 12px;
  }
  table th,
  table td {
    padding: 6px 8px !important;
  }
}
</style>
