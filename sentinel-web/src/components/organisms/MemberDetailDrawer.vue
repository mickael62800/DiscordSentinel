<script setup lang="ts">
import { ref, watch } from "vue";
import { useMembers } from "../../composables/useMembers";
import { useFormatDate } from "../../composables/useFormatDate";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import AppBadge from "../atoms/AppBadge.vue";
import AppTabs from "../molecules/AppTabs.vue";
import MemberProfileTab from "./member-drawer/MemberProfileTab.vue";
import MemberSurveillanceOverview from "./member-drawer/MemberSurveillanceOverview.vue";
import MemberActivityHeatmap from "./member-drawer/MemberActivityHeatmap.vue";
import MemberTopChannels from "./member-drawer/MemberTopChannels.vue";
import MemberActivityTimeline from "./member-drawer/MemberActivityTimeline.vue";
import MemberNotesSection from "./member-drawer/MemberNotesSection.vue";

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
        <MemberProfileTab
          :member="selectedMember.member"
          :stats="selectedMember.stats"
          :infractions="selectedMember.infractions"
          :moderation="selectedMember.moderation"
        />
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

            <MemberSurveillanceOverview :activity="activityTimeline" :dossier="dossier" />

            <MemberActivityHeatmap :activity="activityTimeline" />

            <MemberTopChannels :activity="activityTimeline" />

            <MemberActivityTimeline
              v-if="activityTimeline && activityTimeline.length > 0"
              :activity="activityTimeline"
            />

            <MemberNotesSection v-if="dossier.notes" :notes="dossier.notes" />
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
.detail-date {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  flex-shrink: 0;
}
.detail-row-body { font-size: 13px; color: var(--text-primary); white-space: pre-wrap; word-break: break-word; }
.detail-row-sub { font-size: 11px; color: var(--text-secondary); margin-top: 4px; }

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
</style>
