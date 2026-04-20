<script setup lang="ts">
import { onMounted, watch, ref, computed } from "vue";
import { useMembers } from "../../composables/useMembers";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { usePagination } from "../../composables/usePagination";
import { useFormatDate } from "../../composables/useFormatDate";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";

const { success, error: showError } = useToast();
const { confirm: confirmDialog } = useConfirm();
import ErrorState from "../atoms/ErrorState.vue";
import AppBadge from "../atoms/AppBadge.vue";
import PaginationBar from "../molecules/PaginationBar.vue";

const { formatShortDateTime: fmt } = useFormatDate();
const { selectedGuildId } = useGuildSelector();

const {
  filteredMembers,
  loading,
  error,
  search,
  sortBy,
  selectedMember,
  loadingSummary,
  conductConfig,
  conductLog,
  conductLoading,
  dossier,
  dossierLoading,
  isWatched,
  fetchMembers,
  fetchConductConfig,
  selectMember,
  fetchConductDetail,
  adjustPoints,
  fetchDossier,
  addToWatch,
  removeFromWatch,
  resetMember,
  closeMember,
} = useMembers();

// Tabs: detail
const detailTab = ref<"profil" | "conduite" | "surveillance">("profil");

// Filtre surveillance
const watchFilter = ref<"all" | "watched" | "unwatched">("all");

// Adjust form
const adjustAmount = ref(1);
const adjustReason = ref("");
const adjusting = ref(false);

// Watch actions
const watchAction = ref(false);

const tabFilteredMembers = computed(() => {
  let list = filteredMembers.value.filter((m) => !m.is_bot);
  if (watchFilter.value === "watched") list = list.filter((m) => isWatched(m.user_id));
  if (watchFilter.value === "unwatched") list = list.filter((m) => !isWatched(m.user_id));
  // Surveilles en premier
  return list.sort((a, b) => {
    const aW = isWatched(a.user_id) ? 0 : 1;
    const bW = isWatched(b.user_id) ? 0 : 1;
    return aW - bW;
  });
});

const { currentPage, perPage, totalItems, totalPages, paginatedItems: paginatedMembers } = usePagination(tabFilteredMembers);

onMounted(() => { fetchMembers(); fetchConductConfig(); });
watch(selectedGuildId, () => { closeMember(); fetchMembers(); fetchConductConfig(); });

// Quand on change d'onglet detail, charger les donnees necessaires
watch(detailTab, async (tab) => {
  if (!selectedMember.value) return;
  const userId = selectedMember.value.member.user_id;
  if (tab === "conduite") {
    await fetchConductDetail(userId);
  } else if (tab === "surveillance") {
    await fetchDossier(userId);
  }
});

async function onSelectMember(userId: string) {
  detailTab.value = "profil";
  await selectMember(userId);
}

async function doAdjust(positive: boolean) {
  if (!selectedMember.value || !adjustReason.value) return;
  adjusting.value = true;
  try {
    const amount = positive ? Math.abs(adjustAmount.value) : -Math.abs(adjustAmount.value);
    await adjustPoints(selectedMember.value.member.user_id, amount, adjustReason.value);
    adjustReason.value = "";
    // Refresh summary too
    await selectMember(selectedMember.value.member.user_id);
    success("Points de conduite ajustes avec succes");
  } catch (e) {
    console.error("Erreur ajustement:", e);
    showError("Erreur lors de l'ajustement des points");
  } finally {
    adjusting.value = false;
  }
}

async function toggleWatch() {
  if (!selectedMember.value) return;
  watchAction.value = true;
  try {
    // Try to add — if already watched this will error
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
  const ok = await confirmDialog({ title: "Retirer de la surveillance", message: `Retirer ${selectedMember.value.member.username} de la surveillance ?` });
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

const resetting = ref(false);

async function handleReset() {
  if (!selectedMember.value) return;
  const member = selectedMember.value.member;
  const username = member.display_name || member.username;
  const ok1 = await confirmDialog({
    title: "⚠️ Reinitialiser tout",
    message:
      `Supprimer DEFINITIVEMENT toutes les donnees de moderation pour ${username} ?\n\n` +
      "Cela efface :\n" +
      "• Infractions\n" +
      "• Actions de moderation (warns/mutes/bans)\n" +
      "• Points de conduite + historique\n" +
      "• Strikes\n" +
      "• Notes moderateurs\n" +
      "• Surveillance\n" +
      "• Rappels de sanction\n\n" +
      "Cette action est IRREVERSIBLE.",
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
    // Refresh du membre affiche + du dossier.
    await selectMember(member.user_id);
    if (detailTab.value === "surveillance") {
      await fetchDossier(member.user_id);
    } else if (detailTab.value === "conduite") {
      await fetchConductDetail(member.user_id);
    }
  } catch (e) {
    console.error("Erreur reset membre:", e);
    showError("Erreur lors de la reinitialisation du membre");
  } finally {
    resetting.value = false;
  }
}

// Helpers
function conductColor(points: number, max: number): string {
  const ratio = points / max;
  if (ratio >= 0.8) return "var(--success)";
  if (ratio >= 0.5) return "var(--warning)";
  return "var(--danger)";
}

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
</script>

<template>
  <div class="members-page">
    <!-- Header -->
    <div class="page-header-row">
      <h1>Membres</h1>
      <span v-if="!loading" class="member-count">{{ tabFilteredMembers.length }} membres</span>
    </div>

    <!-- Filters -->
    <div class="filters">
      <input v-model="search" type="text" class="search-input" placeholder="Rechercher par nom ou ID..." />
      <select v-model="watchFilter" class="sort-select">
        <option value="all">Tous les membres</option>
        <option value="watched">Surveilles uniquement</option>
        <option value="unwatched">Non surveilles</option>
      </select>
      <select v-model="sortBy" class="sort-select">
        <option value="username">Tri par nom</option>
        <option value="joined_at">Tri par date d'arrivee</option>
      </select>
    </div>

    <div v-if="loading" class="loading">Chargement...</div>
    <ErrorState v-else-if="error" :message="error" :retryable="true" @retry="fetchMembers" />

    <div v-else class="content-layout">
      <!-- ===== LEFT: Member list ===== -->
      <div class="members-list">
        <div
          v-for="member in paginatedMembers"
          :key="member.user_id"
          :class="['card', 'member-card', { selected: selectedMember?.member.user_id === member.user_id }]"
          @click="onSelectMember(member.user_id)"
        >
          <div class="member-card-header">
            <div class="member-identity">
              <div class="avatar-placeholder member-avatar">{{ member.username.charAt(0).toUpperCase() }}</div>
              <div class="member-names">
                <span class="member-name">{{ member.display_name || member.username }}</span>
                <span class="member-id">{{ member.username }}</span>
              </div>
            </div>
            <div class="member-badges">
              <AppBadge v-if="isWatched(member.user_id)" label="SURVEILLE" variant="warning" />
            </div>
          </div>
          <div class="member-card-footer">
            <span>{{ rolesCount(member.roles) }} roles</span>
            <span>Depuis {{ formatDate(member.joined_at) }}</span>
          </div>
        </div>

        <div v-if="tabFilteredMembers.length === 0" class="empty">Aucun membre trouve</div>

        <PaginationBar
          :current-page="currentPage"
          :total-pages="totalPages"
          :total-items="totalItems"
          :per-page="perPage"
          @update:current-page="currentPage = $event"
          @update:per-page="perPage = $event"
        />
      </div>

      <!-- ===== RIGHT: Detail panel ===== -->
      <div v-if="selectedMember" class="card card--lg detail-panel">
        <div class="panel-top-actions">
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
          <!-- Profile header (always visible) -->
          <div class="profile-header">
            <div class="avatar-placeholder profile-avatar">{{ selectedMember.member.username.charAt(0).toUpperCase() }}</div>
            <div class="profile-info">
              <h2>{{ selectedMember.member.display_name || selectedMember.member.username }}</h2>
              <span class="profile-id">{{ selectedMember.member.user_id }}</span>
            </div>
          </div>

          <!-- Detail tabs -->
          <div class="detail-tabs">
            <button :class="['dtab', { active: detailTab === 'profil' }]" @click="detailTab = 'profil'">Profil</button>
            <button :class="['dtab', { active: detailTab === 'conduite' }]" @click="detailTab = 'conduite'">Conduite</button>
            <button :class="['dtab', { active: detailTab === 'surveillance' }]" @click="detailTab = 'surveillance'">Surveillance</button>
          </div>

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

            <!-- Conduct bar summary -->
            <div class="conduct-mini">
              <div class="conduct-header">
                <span class="section-label">Conduite</span>
                <span class="conduct-value">{{ selectedMember.conduct.points }} / {{ selectedMember.conduct.max_points }}</span>
              </div>
              <div class="conduct-bar-track">
                <div class="conduct-bar-fill" :style="{ width: (selectedMember.conduct.points / selectedMember.conduct.max_points * 100) + '%', backgroundColor: conductColor(selectedMember.conduct.points, selectedMember.conduct.max_points) }" />
              </div>
            </div>

            <!-- Stats -->
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

            <!-- Recent infractions -->
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

          <!-- ── TAB: Conduite ── -->
          <div v-if="detailTab === 'conduite'" class="tab-content">
            <div v-if="conductLoading" class="loading">Chargement...</div>
            <template v-else>
              <!-- Points display -->
              <div class="conduct-display">
                <div class="conduct-big">
                  <span class="points-big" :style="{ color: conductColor(selectedMember.conduct.points, selectedMember.conduct.max_points) }">
                    {{ selectedMember.conduct.points }}
                  </span>
                  <span class="points-max">/ {{ selectedMember.conduct.max_points }}</span>
                </div>
                <div class="conduct-bar-track conduct-bar-lg">
                  <div class="conduct-bar-fill" :style="{ width: (selectedMember.conduct.points / selectedMember.conduct.max_points * 100) + '%', backgroundColor: conductColor(selectedMember.conduct.points, selectedMember.conduct.max_points) }" />
                </div>
              </div>

              <!-- Adjust form -->
              <div class="adjust-section">
                <h3>Ajuster les points</h3>
                <div class="adjust-form">
                  <input v-model.number="adjustAmount" type="number" min="1" max="12" class="adjust-input" />
                  <input v-model="adjustReason" type="text" class="adjust-reason" placeholder="Raison..." />
                  <button class="adjust-btn add" :disabled="adjusting || !adjustReason" @click="doAdjust(true)">+ Ajouter</button>
                  <button class="adjust-btn remove" :disabled="adjusting || !adjustReason" @click="doAdjust(false)">- Retirer</button>
                </div>
              </div>

              <!-- Config summary -->
              <div v-if="conductConfig" class="config-bar">
                <span>Max: {{ conductConfig.max_points }}</span>
                <span>Regen: +{{ conductConfig.regen_amount }}/{{ conductConfig.regen_interval === 'weekly' ? 'sem' : 'mois' }}</span>
                <span>Warn: -{{ conductConfig.penalty_warn }}</span>
                <span>Delete: -{{ conductConfig.penalty_delete }}</span>
                <span>Mute: -{{ conductConfig.penalty_mute }}</span>
                <span>Ban: -{{ conductConfig.penalty_ban }}</span>
              </div>

              <!-- History -->
              <h3>Historique</h3>
              <div v-if="conductLog.length === 0" class="empty-small">Aucun mouvement</div>
              <div v-for="entry in conductLog" :key="entry.id" class="detail-row">
                <div class="detail-row-header">
                  <span class="detail-date">{{ fmt(entry.created_at) }}</span>
                  <span :class="['delta', entry.delta < 0 ? 'delta-neg' : 'delta-pos']">
                    {{ entry.delta > 0 ? '+' : '' }}{{ entry.delta }}
                  </span>
                </div>
                <div class="detail-row-body">{{ entry.reason }}</div>
                <div class="detail-row-sub">{{ entry.points_before }} &rarr; {{ entry.points_after }}</div>
              </div>
            </template>
          </div>

          <!-- ── TAB: Surveillance ── -->
          <div v-if="detailTab === 'surveillance'" class="tab-content">
            <div v-if="dossierLoading" class="loading">Chargement...</div>
            <template v-else>
              <!-- Note : les actions "+ Surveiller" / "Retirer" sont dans
                   panel-top-actions (en haut a droite). On evite ici le
                   doublon. -->
              <template v-if="isWatched(selectedMember.member.user_id) && dossier">
                <!-- Dossier summary -->
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

                <!-- Infractions -->
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

                <!-- Moderation actions -->
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

                <!-- Security events -->
                <div v-if="dossier.security_events.length > 0" class="section">
                  <h3>Evenements de securite ({{ dossier.security_events.length }})</h3>
                  <div v-for="evt in dossier.security_events.slice(0, 10)" :key="evt.id" class="detail-row">
                    <div class="detail-row-header">
                      <span class="detail-date">{{ fmt(evt.created_at) }}</span>
                      <AppBadge :label="evt.severity" :variant="evt.severity === 'critical' ? 'danger' : evt.severity === 'warning' ? 'warning' : 'info'" />
                    </div>
                    <div class="detail-row-body">{{ evt.description }}</div>
                  </div>
                </div>

                <!-- Notes -->
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

      <!-- Placeholder -->
      <div v-else class="card card--xl detail-placeholder">
        <div class="placeholder-icon">&#x1f465;</div>
        <p>Selectionnez un membre pour voir son profil</p>
      </div>
    </div>
  </div>
</template>

<style scoped>
.members-page h1 { margin: 0; }

.page-header-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}

.member-count {
  font-size: 13px;
  color: var(--text-secondary);
  font-weight: 600;
}

/* Tabs */
.tabs, .detail-tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 4px;
}

.detail-tabs {
  background: var(--bg-secondary);
  border: none;
}

.tab, .dtab {
  flex: 1;
  padding: 8px 16px;
  border: none;
  border-radius: 6px;
  background: transparent;
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.tab:hover, .dtab:hover {
  color: var(--text-primary);
  background: var(--bg-hover);
}

.tab.active, .dtab.active {
  background: var(--accent);
  color: white;
}

/* Filters */
.filters {
  display: flex;
  gap: 12px;
  margin-bottom: 20px;
}

.search-input {
  flex: 1;
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
}

.search-input::placeholder { color: var(--text-secondary); }
.search-input:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
}

.sort-select {
  padding: 10px 14px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-card);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  min-width: 180px;
}

.sort-select:focus { outline: none; border-color: var(--accent); }

.loading, .empty { color: var(--text-secondary); padding: 40px; text-align: center; }
.empty-small { color: var(--text-secondary); text-align: center; padding: 20px; font-size: 13px; }

/* Layout */
.content-layout {
  display: flex;
  gap: 20px;
  min-height: 0;
}

/* Left list */
.members-list {
  width: 720px;
  min-width: 720px;
  max-width: 720px;
  display: flex;
  flex-direction: column;
  gap: 8px;
  overflow-y: auto;
  max-height: calc(100vh - 240px);
  padding-right: 4px;
}

.member-card {
  padding: 14px 16px; /* override .card : plus compact que le default */
  cursor: pointer;
  transition: all var(--transition-fast);
}

.member-card:hover { border-color: var(--accent); background: var(--bg-hover); }
.member-card.selected { border-color: var(--accent); box-shadow: var(--focus-ring); }

.member-card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 8px;
}

.member-identity { display: flex; align-items: center; gap: 10px; }

.member-avatar {
  width: 36px;
  height: 36px;
  font-size: 14px;
}

.member-names { display: flex; flex-direction: column; gap: 1px; }
.member-name { font-weight: 600; font-size: 14px; color: var(--text-primary); }
.member-id { font-size: 11px; color: var(--text-secondary); font-family: "JetBrains Mono", "Cascadia Code", monospace; }
.member-badges { display: flex; gap: 6px; }
.member-card-footer { display: flex; gap: 12px; font-size: 11px; color: var(--text-secondary); }

/* Right panel */
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

.unwatch-top-btn:hover:not(:disabled) {
  background: var(--danger);
  color: white;
}

.unwatch-top-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

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

.watch-top-btn:hover:not(:disabled) {
  background: var(--warning);
  color: #0a0a0a;
}

.watch-top-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

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

.reset-top-btn:hover:not(:disabled) {
  background: var(--danger);
  color: white;
}

.reset-top-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

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

.profile-avatar {
  width: 56px;
  height: 56px;
  font-size: 24px;
}

.profile-info h2 { margin: 0; font-size: 20px; }
.profile-id { font-size: 12px; color: var(--text-secondary); font-family: "JetBrains Mono", "Cascadia Code", monospace; }

.tab-content { margin-top: 4px; }

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

.conduct-mini { margin-bottom: 20px; }

.conduct-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 6px;
}

.section-label { font-size: 12px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.3px; font-weight: 600; }
.conduct-value { font-size: 13px; font-weight: 700; color: var(--text-primary); }

.conduct-bar-track {
  width: 100%;
  height: 6px;
  background: var(--bg-secondary);
  border-radius: 3px;
  overflow: hidden;
}

.conduct-bar-lg { height: 10px; border-radius: 5px; }

.conduct-bar-fill {
  height: 100%;
  border-radius: inherit;
  transition: width 0.3s ease;
}

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

.detail-row-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 4px; }
.detail-date { font-size: 11px; color: var(--text-secondary); font-family: "JetBrains Mono", "Cascadia Code", monospace; }
.detail-row-body { font-size: 13px; color: var(--text-primary); }
.detail-row-sub { font-size: 11px; color: var(--text-secondary); margin-top: 4px; }

/* Conduite tab */
.conduct-display { margin-bottom: 20px; text-align: center; }
.conduct-big { margin-bottom: 10px; }
.points-big { font-size: 48px; font-weight: 800; }
.points-max { font-size: 24px; color: var(--text-secondary); margin-left: 4px; }

.adjust-section { margin-bottom: 20px; }
.adjust-section h3 { font-size: 14px; margin-bottom: 10px; }

.adjust-form { display: flex; gap: 8px; align-items: center; }

.adjust-input {
  width: 60px;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 14px;
  text-align: center;
}

.adjust-reason {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 13px;
}

.adjust-reason::placeholder { color: var(--text-secondary); }
.adjust-input:focus, .adjust-reason:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
}

.adjust-btn {
  padding: 8px 14px;
  border: none;
  border-radius: 8px;
  font-size: 12px;
  font-weight: 600;
  cursor: pointer;
  transition: opacity var(--transition-fast);
  white-space: nowrap;
}

.adjust-btn:disabled { opacity: 0.4; cursor: not-allowed; }
.adjust-btn.add { background: var(--success-bg); color: var(--success); border: 1px solid var(--success); }
.adjust-btn.add:hover:not(:disabled) { background: var(--success); color: white; }
.adjust-btn.remove { background: var(--danger-bg); color: var(--danger); border: 1px solid var(--danger); }
.adjust-btn.remove:hover:not(:disabled) { background: var(--danger); color: white; }

.config-bar {
  display: flex;
  gap: 12px;
  padding: 10px 14px;
  background: var(--bg-secondary);
  border-radius: 8px;
  margin-bottom: 16px;
  font-size: 12px;
  color: var(--text-secondary);
  flex-wrap: wrap;
}

.delta { font-weight: 700; font-family: "JetBrains Mono", "Cascadia Code", monospace; }
.delta-pos { color: var(--success); }
.delta-neg { color: var(--danger); }

/* Surveillance tab */
.watch-actions { margin-bottom: 16px; }

.watch-btn {
  padding: 8px 18px;
  border-radius: 8px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-fast);
}

.watch-btn:disabled { opacity: 0.5; cursor: not-allowed; }
.watch-btn.add { background: var(--accent); color: white; border: none; }
.watch-btn.add:hover:not(:disabled) { opacity: 0.85; }
.watch-btn.remove { background: var(--danger-bg); color: var(--danger); border: 1px solid var(--danger); }
.watch-btn.remove:hover:not(:disabled) { background: var(--danger); color: white; }

.dossier-summary {
  display: flex;
  gap: 12px;
  margin-bottom: 20px;
}

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

/* Placeholder */
.detail-placeholder {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--text-secondary);
}

.placeholder-icon { font-size: 48px; margin-bottom: 12px; opacity: 0.5; }
.detail-placeholder p { font-size: 14px; }
</style>
