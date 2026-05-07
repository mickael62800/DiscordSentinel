<script setup lang="ts">
import { ref, computed } from "vue";
import { levelsService } from "@/services/levelsService";
import { useLevels } from "../../composables/useLevels";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import type { UserLevel } from "../../types";
import AppModal from "../atoms/AppModal.vue";
import AppButton from "../atoms/AppButton.vue";
import AppTabs from "../molecules/AppTabs.vue";

const { leaderboard, fetchAll } = useLevels();
const { selectedGuildId } = useGuildSelector();
const { success: toastOk, error: toastErr } = useToast();
const { confirm } = useConfirm();

type ViewMode = "global" | "text" | "voice";
const viewMode = ref<ViewMode>("global");

const viewTabs = [
  { key: "global", label: "Global" },
  { key: "text", label: "Texte" },
  { key: "voice", label: "Vocal" },
];

const editTarget = ref<{ user: UserLevel; mode: ViewMode } | null>(null);
const editXpInput = ref<number>(0);
const editing = ref(false);
const resetting = ref<string | null>(null);

function openEditModal(user: UserLevel, mode: ViewMode) {
  editTarget.value = { user, mode };
  editXpInput.value = mode === "text" ? user.xp_text : mode === "voice" ? user.xp_voice : (user.xp_text + user.xp_voice);
}

function closeEditModal() {
  editTarget.value = null;
  editXpInput.value = 0;
}

async function saveEditXp() {
  if (!editTarget.value || !selectedGuildId.value) return;
  const { user, mode } = editTarget.value;
  if (mode === "global") {
    toastErr("Le total n'est pas editable directement. Modifie l'XP texte ou l'XP vocal.");
    return;
  }
  const xp = Math.max(0, Math.floor(Number(editXpInput.value) || 0));
  editing.value = true;
  try {
    const body: { guild_id: string; user_id: string; xp_text?: number; xp_voice?: number } = {
      guild_id: selectedGuildId.value,
      user_id: user.user_id,
    };
    if (mode === "text") body.xp_text = xp;
    else body.xp_voice = xp;
    await levelsService.setUserXp(body);
    toastOk(`XP ${mode} mis a jour pour ${user.username}.`);
    closeEditModal();
    await fetchAll();
  } catch (e: unknown) {
    toastErr(`Echec edit XP : ${(e as Error)?.message ?? e}`);
  } finally {
    editing.value = false;
  }
}

async function resetUserXp(user: UserLevel, target: "all" | "text" | "voice") {
  if (!selectedGuildId.value) return;
  const labels = { all: "tout (texte + vocal)", text: "le texte", voice: "le vocal" };
  const ok = await confirm({
    title: `Reset ${labels[target]} de ${user.username}`,
    message: `Remettre a 0 ${labels[target]} pour ${user.username} ? Action irreversible.`,
  });
  if (!ok) return;
  resetting.value = `${user.user_id}-${target}`;
  try {
    await levelsService.resetUserXp({
      guild_id: selectedGuildId.value,
      user_id: user.user_id,
      target,
    });
    toastOk(`XP ${target} reset pour ${user.username}.`);
    await fetchAll();
  } catch (e: unknown) {
    toastErr(`Echec reset : ${(e as Error)?.message ?? e}`);
  } finally {
    resetting.value = null;
  }
}

function progressPercent(current: number, needed: number): number {
  if (needed <= 0) return 0;
  return Math.min(100, Math.round((current / needed) * 100));
}

function userLevel(user: UserLevel): number {
  if (viewMode.value === "text") return user.level_text;
  if (viewMode.value === "voice") return user.level_voice;
  return user.level;
}
function userXp(user: UserLevel): number {
  if (viewMode.value === "text") return user.xp_text;
  if (viewMode.value === "voice") return user.xp_voice;
  return user.xp;
}
function userCurrent(user: UserLevel): number {
  if (viewMode.value === "text") return user.xp_text_current;
  if (viewMode.value === "voice") return user.xp_voice_current;
  return user.xp_current;
}
function userNeeded(user: UserLevel): number {
  if (viewMode.value === "text") return user.xp_text_needed;
  if (viewMode.value === "voice") return user.xp_voice_needed;
  return user.xp_needed;
}

const sortedLeaderboard = computed<UserLevel[]>(() =>
  [...leaderboard.value].sort((a, b) => userXp(b) - userXp(a)),
);
</script>

<template>
  <div>
    <AppTabs
      :model-value="viewMode"
      :tabs="viewTabs"
      class="view-tabs-wrap"
      @update:model-value="(k) => (viewMode = k as ViewMode)"
    />

    <div class="leaderboard">
      <div
        v-for="(user, index) in sortedLeaderboard"
        :key="user.id"
        :class="['card', 'user-row', { 'top-3': index < 3 }]"
      >
        <div class="rank">
          <span :class="['rank-number', `rank-${index + 1}`]">{{ index + 1 }}</span>
        </div>
        <div class="avatar-placeholder user-avatar-placeholder">{{ user.username.charAt(0).toUpperCase() }}</div>
        <div class="user-info">
          <div class="user-header">
            <span class="user-name">{{ user.username }}</span>
            <span class="user-level">Niv. {{ userLevel(user) }}</span>
          </div>
          <div class="progress-container">
            <div class="progress-bar">
              <div class="progress-fill" :style="{ width: progressPercent(userCurrent(user), userNeeded(user)) + '%' }"></div>
            </div>
            <span class="progress-text">{{ userCurrent(user) }} / {{ userNeeded(user) }} XP</span>
          </div>
          <div v-if="viewMode === 'global'" class="mini-stats">
            <span class="mini-stat text">Texte Niv.{{ user.level_text }}</span>
            <span class="mini-stat voice">Vocal Niv.{{ user.level_voice }}</span>
          </div>
        </div>
        <div class="user-xp">
          <span class="xp-total">{{ userXp(user).toLocaleString() }}</span>
          <span class="xp-label">XP {{ viewMode === 'text' ? 'texte' : viewMode === 'voice' ? 'vocal' : 'total' }}</span>
        </div>
        <div class="user-actions">
          <button
            v-if="viewMode !== 'global'"
            class="action-btn edit"
            :title="`Modifier l'XP ${viewMode === 'text' ? 'texte' : 'vocal'} de cet utilisateur`"
            @click="openEditModal(user, viewMode)"
          >
            ✎ Edit
          </button>
          <button
            class="action-btn reset"
            :disabled="resetting === `${user.user_id}-${viewMode === 'global' ? 'all' : viewMode}`"
            :title="`Remettre a 0 l'XP ${viewMode === 'global' ? 'total' : viewMode === 'text' ? 'texte' : 'vocal'}`"
            @click="resetUserXp(user, viewMode === 'global' ? 'all' : viewMode)"
          >
            ↺ Reset
          </button>
        </div>
      </div>

      <div v-if="leaderboard.length === 0" class="empty">
        Aucun membre n'a encore d'XP sur ce serveur.
      </div>
    </div>

    <AppModal
      :visible="!!editTarget"
      size="md"
      @close="closeEditModal"
    >
      <template #header>
        <h3>✎ Modifier l'XP de <strong>{{ editTarget?.user.username }}</strong></h3>
      </template>

      <p class="modal-hint">
        Champ : <strong>XP {{ editTarget?.mode === 'text' ? 'texte' : editTarget?.mode === 'voice' ? 'vocal' : 'total (réparti texte+vocal)' }}</strong>.
        Le niveau correspondant sera recalculé automatiquement.
      </p>
      <label class="modal-field">
        <span>Nouvelle valeur XP</span>
        <input
          v-model.number="editXpInput"
          type="number"
          min="0"
          step="1"
          :disabled="editing"
        />
      </label>

      <template #footer>
        <AppButton variant="secondary" :disabled="editing" @click="closeEditModal">Annuler</AppButton>
        <AppButton variant="primary" :disabled="editing" @click="saveEditXp">
          {{ editing ? 'Enregistrement…' : 'Enregistrer' }}
        </AppButton>
      </template>
    </AppModal>
  </div>
</template>

<style scoped>
.view-tabs-wrap { margin-bottom: 16px; }

.leaderboard {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.user-row {
  padding: 14px 18px;
  display: flex;
  align-items: center;
  gap: 14px;
}
.user-row.top-3 { border-color: rgba(88, 101, 242, 0.3); }

.rank { width: 32px; text-align: center; }
.rank-number { font-weight: 700; font-size: 16px; color: var(--text-secondary); }
.rank-1 { color: #FFD700; }
.rank-2 { color: #C0C0C0; }
.rank-3 { color: #CD7F32; }

.user-avatar-placeholder { width: 40px; height: 40px; font-size: 16px; }

.user-info { flex: 1; min-width: 0; }
.user-header { display: flex; align-items: center; gap: 8px; margin-bottom: 6px; }
.user-name { font-weight: 600; font-size: 14px; color: var(--text-primary); }
.user-level {
  font-size: 12px;
  font-weight: 600;
  color: var(--accent);
  background-color: var(--accent-bg);
  padding: 2px 8px;
  border-radius: 4px;
}

.progress-container { display: flex; align-items: center; gap: 10px; }
.progress-bar {
  flex: 1; height: 8px;
  background-color: var(--bg-hover);
  border-radius: 4px; overflow: hidden;
}
.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), var(--accent-alt));
  border-radius: 4px;
  transition: width 0.3s;
}
.progress-text {
  font-size: 11px; color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  white-space: nowrap;
  min-width: 100px;
  text-align: right;
}

.mini-stats { display: flex; gap: 8px; margin-top: 4px; }
.mini-stat {
  font-size: 10px; font-weight: 600;
  padding: 1px 6px; border-radius: 3px;
}
.mini-stat.text { color: #3498DB; background: rgba(52, 152, 219, 0.1); }
.mini-stat.voice { color: #E91E63; background: rgba(233, 30, 99, 0.1); }

.user-xp {
  display: flex; flex-direction: column; align-items: flex-end;
  gap: 2px; min-width: 80px;
}
.xp-total {
  font-weight: 700; font-size: 14px;
  color: var(--text-primary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}
.xp-label { font-size: 10px; color: var(--text-secondary); text-transform: uppercase; }

.user-actions { display: flex; flex-direction: column; gap: 4px; margin-left: 12px; }
.action-btn {
  font-size: 11px; font-weight: 600;
  padding: 5px 10px;
  border-radius: 6px; cursor: pointer;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-secondary);
  transition: color 0.15s, border-color 0.15s, background-color 0.15s;
  white-space: nowrap;
}
.action-btn.edit:hover {
  color: var(--accent); border-color: var(--accent);
  background: color-mix(in srgb, var(--accent) 10%, transparent);
}
.action-btn.reset {
  color: var(--danger, #ef4444);
  border-color: color-mix(in srgb, var(--danger, #ef4444) 40%, var(--border));
}
.action-btn.reset:hover:not(:disabled) {
  background: color-mix(in srgb, var(--danger, #ef4444) 12%, transparent);
  border-color: var(--danger, #ef4444);
}
.action-btn:disabled { opacity: 0.5; cursor: not-allowed; }

.empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}

.modal-hint { font-size: 13px; color: var(--text-secondary); margin: 0 0 16px 0; }
.modal-field { display: flex; flex-direction: column; gap: 6px; font-size: 13px; font-weight: 600; }
.modal-field input {
  padding: 10px 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 8px;
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
  font-size: 14px;
}
.modal-field input:focus { outline: none; border-color: var(--accent); }

@media (max-width: 600px) {
  .user-row {
    padding: 10px 12px;
    gap: 10px;
    flex-wrap: wrap;
  }
  .rank { width: 24px; }
  .rank-number { font-size: 14px; }
  .user-avatar-placeholder { width: 32px; height: 32px; font-size: 14px; }
  .user-info { flex: 1 1 calc(100% - 100px); }
  .progress-text { min-width: 0; font-size: 10px; }
  .user-xp { min-width: 0; flex-shrink: 0; }
  .xp-total { font-size: 13px; }
  .user-actions {
    flex-direction: row;
    margin-left: 0;
    flex: 0 0 100%;
    justify-content: flex-end;
  }
  .action-btn { padding: 4px 8px; font-size: 10px; }
  .mini-stats { gap: 4px; }
}
</style>
