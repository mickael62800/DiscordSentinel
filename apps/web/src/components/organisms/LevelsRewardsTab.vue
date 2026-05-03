<script setup lang="ts">
import { ref, computed } from "vue";
import { useLevels } from "../../composables/useLevels";
import type { DiscordRole } from "../../types";

const { rewards, roles, setReward, deleteReward } = useLevels();

const roleSearch = ref("");
const saving = ref<string | null>(null);
const converterLevel = ref<number>(1);

const filteredRoles = computed<DiscordRole[]>(() => {
  let list = roles.value
    .filter((r: DiscordRole) => r.name !== "@everyone" && !r.managed)
    .sort((a: DiscordRole, b: DiscordRole) => b.position - a.position);
  if (roleSearch.value) {
    const q = roleSearch.value.toLowerCase();
    list = list.filter((r) => r.name.toLowerCase().includes(q));
  }
  return list;
});

function getRewardLevel(roleId: string, source: string): number | null {
  const r = rewards.value.find((rw) => rw.role_id === roleId && rw.source === source);
  return r ? r.level : null;
}

async function updateReward(roleId: string, source: string, levelStr: string) {
  const level = parseInt(levelStr);
  saving.value = `${roleId}-${source}`;
  try {
    if (!levelStr || isNaN(level) || level <= 0) {
      const existing = rewards.value.find((rw) => rw.role_id === roleId && rw.source === source);
      if (existing) await deleteReward(existing.level, source);
    } else {
      const existing = rewards.value.find((rw) => rw.role_id === roleId && rw.source === source);
      if (existing && existing.level !== level) await deleteReward(existing.level, source);
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

// ── Convertisseur Niveau <-> Heures ──
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
  const hours = xp / 300;
  return hours < 1 ? `${Math.round(hours * 60)}min` : `${hours.toFixed(1)}h`;
}
function levelToHoursText(level: number): string {
  const xp = cumulativeXp(level);
  const hours = xp / 500;
  return hours < 1 ? `${Math.round(hours * 60)}min` : `${hours.toFixed(1)}h`;
}
function levelToXp(level: number): string {
  return cumulativeXp(level).toLocaleString();
}
</script>

<template>
  <div>
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
                  type="number" min="0" class="level-input" placeholder="-"
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
                  type="number" min="0" class="level-input" placeholder="-"
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
                  type="number" min="0" class="level-input" placeholder="-"
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
  </div>
</template>

<style scoped>
.converter-box {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
  padding: 16px 20px;
  margin-bottom: 20px;
}
.converter-title {
  font-size: 13px; font-weight: 600;
  color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: 0.5px;
  margin: 0 0 12px 0;
}
.converter-row { display: flex; align-items: center; gap: 16px; }
.converter-label { font-size: 14px; font-weight: 600; color: var(--text-primary); white-space: nowrap; }
.converter-input {
  width: 80px;
  padding: 8px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: 13px; font-weight: 700;
  text-align: center;
  font-family: "JetBrains Mono", monospace;
  -moz-appearance: textfield;
}
.converter-input::-webkit-inner-spin-button,
.converter-input::-webkit-outer-spin-button {
  -webkit-appearance: none; margin: 0;
}
.converter-input:focus { border-color: var(--accent); outline: none; }

.converter-results { display: flex; gap: 16px; flex: 1; }
.converter-result {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 8px 16px;
  display: flex; flex-direction: column;
  align-items: center; gap: 2px;
  min-width: 100px;
}
.converter-result.text { border-left: 3px solid #3498DB; }
.converter-result.voice { border-left: 3px solid #E91E63; }
.converter-result-label {
  font-size: 10px; color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: 0.3px;
}
.converter-result-value {
  font-size: 16px; font-weight: 700;
  color: var(--text-primary);
  font-family: "JetBrains Mono", monospace;
}

.rewards-header {
  display: flex; justify-content: space-between; align-items: center;
  margin-bottom: 16px; gap: 16px;
}
.rewards-desc { color: var(--text-secondary); font-size: 13px; flex: 1; }
.role-search {
  background: var(--bg-card);
  border: 1px solid var(--border);
  color: var(--text-primary);
  padding: 8px 12px;
  border-radius: 6px;
  font-size: 13px;
  width: 220px;
}

.rewards-table { overflow-x: auto; }
.rewards-table table { width: 100%; border-collapse: collapse; }
.rewards-table th {
  text-align: left;
  padding: 10px 12px;
  font-size: 11px;
  text-transform: uppercase; letter-spacing: 0.5px;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
}
.rewards-table td {
  padding: 8px 12px;
  font-size: 13px;
  border-bottom: 1px solid var(--border);
}
.rewards-table tr:hover { background: var(--bg-secondary); }
.col-level { width: 140px; }

.role-cell { display: flex; align-items: center; gap: 8px; }
.role-dot { width: 12px; height: 12px; border-radius: 50%; flex-shrink: 0; }
.role-name { font-weight: 600; }

.level-input-wrap { position: relative; display: inline-flex; align-items: center; }
.level-input {
  width: 70px;
  padding: 8px 12px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  color: var(--text-primary);
  font-size: 13px; text-align: center;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}
.level-input:focus { border-color: var(--accent); outline: none; }

.saving-indicator {
  width: 8px; height: 8px;
  border-radius: 50%;
  background: var(--accent);
  margin-left: 6px;
  animation: pulse 0.8s infinite;
}
@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.3; }
}

.mono { font-family: monospace; }
.empty { color: var(--text-secondary); padding: 40px; text-align: center; }

@media (max-width: 768px) {
  .converter-row { flex-direction: column; align-items: stretch; gap: 8px; }
  .converter-input { width: 100%; }
}
</style>
