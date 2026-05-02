<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { botConfigService } from "@/services/botConfigService";
import { levelsService } from "@/services/levelsService";
import { useLevels } from "../../composables/useLevels";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useToast } from "../../composables/useToast";
import { useConfirm } from "../../composables/useConfirm";
import ErrorState from "../atoms/ErrorState.vue";
import type { UserLevel, DiscordRole } from "../../types";
import { useRealtimeRefresh } from "../../composables/useRealtimeRefresh";

const { config, leaderboard, rewards, roles, loading, error, fetchAll, setReward, deleteReward } = useLevels();
const { selectedGuildId } = useGuildSelector();
const { success: toastOk, error: toastErr } = useToast();
const { confirm } = useConfirm();
useRealtimeRefresh(["xp_gained", "xp_admin_set", "xp_admin_reset"], fetchAll);

// ── Admin overrides : edit XP / reset ──
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
  const xp = Math.max(0, Math.floor(Number(editXpInput.value) || 0));
  editing.value = true;
  try {
    const body: { guild_id: string; user_id: string; xp_text?: number; xp_voice?: number } = {
      guild_id: selectedGuildId.value,
      user_id: user.user_id,
    };
    if (mode === "text") body.xp_text = xp;
    else if (mode === "voice") body.xp_voice = xp;
    else {
      // global : repartit equitablement
      body.xp_text = Math.floor(xp / 2);
      body.xp_voice = xp - Math.floor(xp / 2);
    }
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

type ViewMode = "global" | "text" | "voice";
type PageTab = "leaderboard" | "rewards";
const viewMode = ref<ViewMode>("global");
const pageTab = ref<PageTab>("leaderboard");

// Mode de calcul XP pour les roles
const xpRoleMode = ref<string>("separate");
const xpRoleModeLoading = ref(false);

async function loadXpRoleMode() {
  if (!selectedGuildId.value) return;
  try {
    const configs = await botConfigService.getGuildConfig(selectedGuildId.value);
    const found = configs.find((c) => c.config_key === "xp_role_mode");
    xpRoleMode.value = found?.config_value ?? "separate";
  } catch {
    xpRoleMode.value = "separate";
  }
}

async function saveXpRoleMode(mode: string) {
  if (!selectedGuildId.value) return;
  xpRoleModeLoading.value = true;
  try {
    await botConfigService.set(selectedGuildId.value, "progression", "xp_role_mode", mode);
    xpRoleMode.value = mode;
  } catch (e) {
    console.error("Erreur sauvegarde xp_role_mode:", e);
  } finally {
    xpRoleModeLoading.value = false;
  }
}

watch(selectedGuildId, loadXpRoleMode, { immediate: true });

const roleSearch = ref("");
const saving = ref<string | null>(null);

// Roles filtres (exclure @everyone et les roles bot-managed)
const filteredRoles = computed(() => {
  let list = roles.value
    .filter((r: DiscordRole) => r.name !== "@everyone" && !r.managed)
    .sort((a: DiscordRole, b: DiscordRole) => b.position - a.position);
  if (roleSearch.value) {
    const q = roleSearch.value.toLowerCase();
    list = list.filter((r) => r.name.toLowerCase().includes(q));
  }
  return list;
});

// Trouver le reward pour un role + source
function getRewardLevel(roleId: string, source: string): number | null {
  const r = rewards.value.find((rw) => rw.role_id === roleId && rw.source === source);
  return r ? r.level : null;
}

// Mettre a jour un reward
async function updateReward(roleId: string, source: string, levelStr: string) {
  const level = parseInt(levelStr);
  saving.value = `${roleId}-${source}`;
  try {
    if (!levelStr || isNaN(level) || level <= 0) {
      // Supprimer le reward existant
      const existing = rewards.value.find((rw) => rw.role_id === roleId && rw.source === source);
      if (existing) {
        await deleteReward(existing.level, source);
      }
    } else {
      // Supprimer l'ancien si le niveau a change
      const existing = rewards.value.find((rw) => rw.role_id === roleId && rw.source === source);
      if (existing && existing.level !== level) {
        await deleteReward(existing.level, source);
      }
      await setReward(level, roleId, source);
    }
  } catch (e) {
    console.error("Erreur mise a jour reward:", e);
  } finally {
    saving.value = null;
  }
}

function roleColor(color: number): string {
  if (color === 0) return "var(--text-secondary)";
  return `#${color.toString(16).padStart(6, "0")}`;
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

function sortedLeaderboard(): UserLevel[] {
  return [...leaderboard.value].sort((a, b) => userXp(b) - userXp(a));
}

// ── Convertisseur Niveau <-> Heures ──
const converterLevel = ref<number>(1);

function xpForLevel(l: number): number {
  if (l <= 0) return 0;
  return 5 * l * l + 50 * l + 100;
}

function cumulativeXp(level: number): number {
  let total = 0;
  for (let l = 1; l <= level; l++) total += xpForLevel(l);
  return total;
}

function levelToHoursVoice(level: number): string {
  const xp = cumulativeXp(level);
  const hours = xp / 300; // 5 XP/min = 300 XP/h
  return hours < 1 ? `${Math.round(hours * 60)}min` : `${hours.toFixed(1)}h`;
}

function levelToHoursText(level: number): string {
  const xp = cumulativeXp(level);
  const hours = xp / 500; // ~500 XP/h realiste
  return hours < 1 ? `${Math.round(hours * 60)}min` : `${hours.toFixed(1)}h`;
}

function levelToXp(level: number): string {
  return cumulativeXp(level).toLocaleString();
}
</script>

<template>
  <div class="levels">
    <h1>Niveaux & XP</h1>

    <ErrorState v-if="error" :message="error" :retryable="true" @retry="fetchAll" />
    <div v-else-if="loading" class="loading">Chargement...</div>

    <template v-else>
      <!-- Config resume -->
      <div v-if="config" class="config-bar">
        <div class="config-item">
          <span class="config-value">{{ config.xp_per_message }}</span>
          <span class="config-label">XP / message</span>
        </div>
        <div class="config-item">
          <span class="config-value">{{ config.xp_per_voice_minute }}</span>
          <span class="config-label">XP / min vocal</span>
        </div>
        <div class="config-item">
          <span class="config-value">{{ config.xp_cooldown_secs }}s</span>
          <span class="config-label">Cooldown</span>
        </div>
        <div class="config-item">
          <span :class="['config-value', config.enabled ? 'text-success' : 'text-danger']">
            {{ config.enabled ? "Actif" : "Inactif" }}
          </span>
          <span class="config-label">Statut</span>
        </div>
        <div v-if="rewards.length > 0" class="config-item">
          <span class="config-value">{{ rewards.length }}</span>
          <span class="config-label">Recompenses</span>
        </div>
      </div>

      <!-- Mode de calcul XP -->
      <div class="xp-mode-bar">
        <span class="xp-mode-label">Mode d'attribution des roles :</span>
        <select
          class="xp-mode-select"
          :value="xpRoleMode"
          :disabled="xpRoleModeLoading"
          @change="saveXpRoleMode(($event.target as HTMLSelectElement).value)"
        >
          <option value="separate">Separe (texte = niveau texte, vocal = niveau vocal)</option>
          <option value="max">Le plus grand (max entre texte et vocal)</option>
          <option value="total">Total (XP texte + vocal combines)</option>
        </select>
      </div>

      <!-- Page tabs -->
      <div class="page-tabs">
        <button :class="['page-tab', { active: pageTab === 'leaderboard' }]" @click="pageTab = 'leaderboard'">
          Classement
        </button>
        <button :class="['page-tab', { active: pageTab === 'rewards' }]" @click="pageTab = 'rewards'">
          Roles par niveau
        </button>
      </div>

      <!-- ===== LEADERBOARD ===== -->
      <template v-if="pageTab === 'leaderboard'">
        <!-- View mode tabs -->
        <div class="view-tabs">
          <button :class="['tab', { active: viewMode === 'global' }]" @click="viewMode = 'global'">Global</button>
          <button :class="['tab tab-text', { active: viewMode === 'text' }]" @click="viewMode = 'text'">Texte</button>
          <button :class="['tab tab-voice', { active: viewMode === 'voice' }]" @click="viewMode = 'voice'">Vocal</button>
        </div>

        <div class="leaderboard">
          <div
            v-for="(user, index) in sortedLeaderboard()"
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
                class="action-btn edit"
                title="Modifier l'XP de cet utilisateur (selon l'onglet courant)"
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
      </template>

      <!-- ===== REWARDS ===== -->
      <template v-if="pageTab === 'rewards'">
          <!-- Convertisseur Niveau <-> Heures -->
          <div class="converter-box">
            <h3 class="converter-title">Convertisseur Niveau / Heures</h3>
            <div class="converter-row">
              <label class="converter-label">Niveau :</label>
              <input
                v-model.number="converterLevel"
                type="number"
                min="1"
                max="200"
                class="converter-input"
              />
              <div class="converter-results">
                <div class="converter-result">
                  <span class="converter-result-label">XP cumule</span>
                  <span class="converter-result-value">{{ levelToXp(converterLevel) }}</span>
                </div>
                <div class="converter-result text">
                  <span class="converter-result-label">Texte</span>
                  <span class="converter-result-value">{{ levelToHoursText(converterLevel) }}</span>
                </div>
                <div class="converter-result voice">
                  <span class="converter-result-label">Vocal</span>
                  <span class="converter-result-value">{{ levelToHoursVoice(converterLevel) }}</span>
                </div>
              </div>
            </div>
          </div>

          <div class="rewards-header">
            <p class="rewards-desc">
              Definissez le niveau texte et/ou vocal requis pour obtenir chaque role. Laissez vide pour les roles non lies a l'XP.
            </p>
            <input v-model="roleSearch" class="role-search" placeholder="Rechercher un role..." />
          </div>

          <div class="rewards-table">
            <table>
              <thead>
                <tr>
                  <th>Role</th>
                  <th>Membres</th>
                  <th class="col-level">Niveau Texte</th>
                  <th class="col-level">Niveau Vocal</th>
                  <th class="col-level">Jours</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="role in filteredRoles" :key="role.id">
                  <td>
                    <div class="role-cell">
                      <span class="role-dot" :style="{ background: roleColor(role.color) }"></span>
                      <span class="role-name">{{ role.name }}</span>
                    </div>
                  </td>
                  <td class="mono">{{ role.member_count }}</td>
                  <td>
                    <div class="level-input-wrap">
                      <input
                        type="number"
                        min="0"
                        class="level-input"
                        placeholder="-"
                        :value="getRewardLevel(role.id, 'text') ?? ''"
                        :disabled="saving !== null"
                        @change="updateReward(role.id, 'text', ($event.target as HTMLInputElement).value)"
                      />
                      <span v-if="saving === `${role.id}-text`" class="saving-indicator"></span>
                    </div>
                  </td>
                  <td>
                    <div class="level-input-wrap">
                      <input
                        type="number"
                        min="0"
                        class="level-input"
                        placeholder="-"
                        :value="getRewardLevel(role.id, 'voice') ?? ''"
                        :disabled="saving !== null"
                        @change="updateReward(role.id, 'voice', ($event.target as HTMLInputElement).value)"
                      />
                      <span v-if="saving === `${role.id}-voice`" class="saving-indicator"></span>
                    </div>
                  </td>
                  <td>
                    <div class="level-input-wrap">
                      <input
                        type="number"
                        min="0"
                        class="level-input"
                        placeholder="-"
                        :value="getRewardLevel(role.id, 'days') ?? ''"
                        :disabled="saving !== null"
                        @change="updateReward(role.id, 'days', ($event.target as HTMLInputElement).value)"
                      />
                      <span v-if="saving === `${role.id}-days`" class="saving-indicator"></span>
                    </div>
                  </td>
                </tr>
              </tbody>
            </table>
          </div>

          <div v-if="filteredRoles.length === 0" class="empty">
            Aucun role trouve. Les roles sont synchronises par le bot communaute.
          </div>
      </template>
    </template>

    <!-- Modale Edit XP -->
    <div v-if="editTarget" class="modal-overlay" @click.self="closeEditModal">
      <div class="modal-card">
        <header class="modal-head">
          <h3>
            ✎ Modifier l'XP de <strong>{{ editTarget.user.username }}</strong>
          </h3>
          <button class="modal-close" @click="closeEditModal">×</button>
        </header>
        <div class="modal-body">
          <p class="modal-hint">
            Champ : <strong>XP {{ editTarget.mode === 'text' ? 'texte' : editTarget.mode === 'voice' ? 'vocal' : 'total (réparti texte+vocal)' }}</strong>.
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
        </div>
        <footer class="modal-foot">
          <button class="btn-secondary" :disabled="editing" @click="closeEditModal">Annuler</button>
          <button class="btn-primary" :disabled="editing" @click="saveEditXp">
            {{ editing ? 'Enregistrement…' : 'Enregistrer' }}
          </button>
        </footer>
      </div>
    </div>
  </div>
</template>

<style scoped>
.levels h1 {
  margin-bottom: 20px;
}

/* Convertisseur */
.converter-box {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 16px 20px;
  margin-bottom: 20px;
}

.converter-title {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
  margin: 0 0 12px 0;
}

.converter-row {
  display: flex;
  align-items: center;
  gap: 16px;
}

.converter-label {
  font-size: 14px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
}

.converter-input {
  width: 80px;
  padding: 8px 12px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: 16px;
  font-weight: 700;
  text-align: center;
  font-family: "JetBrains Mono", monospace;
  -moz-appearance: textfield;
}

.converter-input::-webkit-inner-spin-button,
.converter-input::-webkit-outer-spin-button {
  -webkit-appearance: none;
  margin: 0;
}

.converter-input:focus {
  border-color: var(--accent);
  outline: none;
}

.converter-results {
  display: flex;
  gap: 16px;
  flex: 1;
}

.converter-result {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 8px 16px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 2px;
  min-width: 100px;
}

.converter-result.text {
  border-left: 3px solid #3498DB;
}

.converter-result.voice {
  border-left: 3px solid #E91E63;
}

.converter-result-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.converter-result-value {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
}

/* Config bar */
.config-bar {
  display: flex;
  gap: 16px;
  margin-bottom: 16px;
  flex-wrap: wrap;
}

.config-item {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 12px 20px;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
  min-width: 100px;
}

.config-value {
  font-weight: 700;
  font-size: 18px;
  color: var(--text-primary);
}

.config-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.3px;
}

.text-success { color: var(--success); }
.text-danger { color: var(--danger); }

/* XP mode */
.xp-mode-bar {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 16px;
  padding: 10px 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
}

.xp-mode-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-secondary);
  white-space: nowrap;
}

.xp-mode-select {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  color: var(--text-primary);
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 13px;
  flex: 1;
  max-width: 450px;
}

/* Page tabs */
.page-tabs {
  display: flex;
  gap: 0;
  border-bottom: 1px solid var(--border);
  margin-bottom: 16px;
}

.page-tab {
  padding: 10px 20px;
  background: none;
  border: none;
  color: var(--text-secondary);
  font-size: 14px;
  cursor: pointer;
  border-bottom: 2px solid transparent;
  transition: all var(--transition-base);
}

.page-tab.active {
  color: var(--accent);
  border-bottom-color: var(--accent);
}

.page-tab:hover:not(.active) {
  color: var(--text-primary);
}

/* View tabs */
.view-tabs {
  display: flex;
  gap: 8px;
  margin-bottom: 16px;
}

.tab {
  padding: 8px 20px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-card);
  color: var(--text-secondary);
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  transition: all var(--transition-base);
}

.tab:hover {
  background: var(--bg-hover);
}

.tab.active {
  background: var(--accent);
  color: white;
  border-color: var(--accent);
}

.tab-text.active {
  background: #3498DB;
  border-color: #3498DB;
}

.tab-voice.active {
  background: #E91E63;
  border-color: #E91E63;
}

/* Leaderboard */
.leaderboard {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.user-row {
  padding: 14px 18px; /* override .card : padding horizontal plus large pour listings */
  display: flex;
  align-items: center;
  gap: 14px;
}

.user-row.top-3 {
  border-color: rgba(88, 101, 242, 0.3);
}

.rank {
  width: 32px;
  text-align: center;
}

.rank-number {
  font-weight: 700;
  font-size: 16px;
  color: var(--text-secondary);
}

.rank-1 { color: #FFD700; }
.rank-2 { color: #C0C0C0; }
.rank-3 { color: #CD7F32; }

.user-avatar-placeholder {
  width: 40px;
  height: 40px;
  font-size: 16px;
}

.user-info {
  flex: 1;
  min-width: 0;
}

.user-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 6px;
}

.user-name {
  font-weight: 600;
  font-size: 14px;
  color: var(--text-primary);
}

.user-level {
  font-size: 12px;
  font-weight: 600;
  color: var(--accent);
  background-color: var(--accent-bg);
  padding: 2px 8px;
  border-radius: 4px;
}

.progress-container {
  display: flex;
  align-items: center;
  gap: 10px;
}

.progress-bar {
  flex: 1;
  height: 8px;
  background-color: var(--bg-hover);
  border-radius: 4px;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: linear-gradient(90deg, var(--accent), var(--accent-alt));
  border-radius: 4px;
  transition: width 0.3s;
}

.progress-text {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  white-space: nowrap;
  min-width: 100px;
  text-align: right;
}

/* Mini stats for global view */
.mini-stats {
  display: flex;
  gap: 8px;
  margin-top: 4px;
}

.mini-stat {
  font-size: 10px;
  font-weight: 600;
  padding: 1px 6px;
  border-radius: 3px;
}

.mini-stat.text {
  color: #3498DB;
  background: rgba(52, 152, 219, 0.1);
}

.mini-stat.voice {
  color: #E91E63;
  background: rgba(233, 30, 99, 0.1);
}

.user-xp {
  display: flex;
  flex-direction: column;
  align-items: flex-end;
  gap: 2px;
  min-width: 80px;
}

.xp-total {
  font-weight: 700;
  font-size: 14px;
  color: var(--text-primary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.xp-label {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
}

/* Rewards section */
.rewards-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
  gap: 16px;
}

.rewards-desc {
  color: var(--text-secondary);
  font-size: 13px;
  flex: 1;
}

.role-search {
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-primary);
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  width: 220px;
}

.rewards-table {
  overflow-x: auto;
}

.rewards-table table {
  width: 100%;
  border-collapse: collapse;
}

.rewards-table th {
  text-align: left;
  padding: 10px 12px;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
}

.rewards-table td {
  padding: 8px 12px;
  font-size: 13px;
  border-bottom: 1px solid var(--border);
}

.rewards-table tr:hover {
  background: var(--bg-secondary);
}

.col-level {
  width: 140px;
}

.role-cell {
  display: flex;
  align-items: center;
  gap: 8px;
}

.role-dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  flex-shrink: 0;
}

.role-name {
  font-weight: 600;
}

.level-input-wrap {
  position: relative;
  display: inline-flex;
  align-items: center;
}

.level-input {
  width: 70px;
  padding: 6px 8px;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: 13px;
  text-align: center;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.level-input:focus {
  border-color: var(--accent);
  outline: none;
}

.saving-indicator {
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: var(--accent);
  margin-left: 6px;
  animation: pulse 0.8s infinite;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.mono {
  font-family: monospace;
}

.loading, .empty {
  color: var(--text-secondary);
  padding: 40px;
  text-align: center;
}

/* ── Boutons admin Edit / Reset par ligne user ── */
.user-actions {
  display: flex;
  flex-direction: column;
  gap: 4px;
  margin-left: 12px;
}
.action-btn {
  font-size: 11px;
  font-weight: 600;
  padding: 5px 10px;
  border-radius: 6px;
  cursor: pointer;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-secondary);
  transition: color 0.15s, border-color 0.15s, background-color 0.15s;
  white-space: nowrap;
}
.action-btn.edit:hover { color: var(--accent); border-color: var(--accent); background: color-mix(in srgb, var(--accent) 10%, transparent); }
.action-btn.reset { color: var(--danger, #ef4444); border-color: color-mix(in srgb, var(--danger, #ef4444) 40%, var(--border)); }
.action-btn.reset:hover:not(:disabled) { background: color-mix(in srgb, var(--danger, #ef4444) 12%, transparent); border-color: var(--danger, #ef4444); }
.action-btn:disabled { opacity: 0.5; cursor: not-allowed; }

/* ── Modale Edit XP ── */
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
  width: 100%; max-width: 480px;
  display: flex; flex-direction: column;
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.5);
}
.modal-head {
  display: flex; justify-content: space-between; align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
}
.modal-head h3 { margin: 0; font-size: 16px; }
.modal-close {
  background: transparent; border: 0; cursor: pointer;
  font-size: 24px; line-height: 1; color: var(--text-secondary);
  padding: 0 6px;
}
.modal-close:hover { color: var(--text-primary); }
.modal-body { padding: 20px; }
.modal-hint { font-size: 13px; color: var(--text-secondary); margin: 0 0 16px 0; }
.modal-field {
  display: flex; flex-direction: column; gap: 6px;
  font-size: 13px; font-weight: 600;
}
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
.modal-foot {
  display: flex; justify-content: flex-end; gap: 10px;
  padding: 14px 20px; border-top: 1px solid var(--border);
}
.modal-foot button {
  padding: 8px 16px; border-radius: 8px; cursor: pointer;
  font-size: 13px; font-weight: 600; border: 1px solid var(--border);
}
.modal-foot button:disabled { opacity: 0.5; cursor: not-allowed; }
.btn-secondary { background: transparent; color: var(--text-primary); }
.btn-secondary:hover:not(:disabled) { background: var(--bg-hover); }
.btn-primary {
  background: var(--accent); color: white; border-color: var(--accent);
}
.btn-primary:hover:not(:disabled) { filter: brightness(1.1); }
</style>
