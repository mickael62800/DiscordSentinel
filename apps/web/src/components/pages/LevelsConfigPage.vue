<script setup lang="ts">
import { onMounted, reactive, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { levelsService, type AddXpPayload } from "@/services/levelsService";
import { conductService } from "@/services/conductService";
import { botConfigService } from "@/services/botConfigService";
import type { LevelConfig, ConductConfig } from "@/types";

const PROGRESSION_BOT = "progression-bot";

const { guildIdFilter } = useGuildSelector();
const { success, error: showError } = useToast();

// ── Levels ──────────────────────────────────────────────
const levelsCfg = ref<LevelConfig | null>(null);
const levelsDraft = reactive({
  xp_per_message: 15,
  xp_per_voice_minute: 5,
  xp_cooldown_secs: 60,
  level_up_channel_id: "",
  level_up_message: "",
  excluded_channels: "",
  enabled: true,
});
const savingLevels = ref(false);

// ── Multiplicateurs XP (lus/ecrits dans bot_guild_config bot=progression-bot) ──
const multipliersDraft = reactive({
  xp_channel_multipliers: "",
  xp_role_multipliers: "",
});
const savingMultipliers = ref(false);

async function fetchMultipliers() {
  if (!guildIdFilter.value) return;
  try {
    const all = await botConfigService.getGuildConfig(guildIdFilter.value);
    const ch = all.find((c) => c.bot_name === PROGRESSION_BOT && c.config_key === "xp_channel_multipliers");
    const ro = all.find((c) => c.bot_name === PROGRESSION_BOT && c.config_key === "xp_role_multipliers");
    multipliersDraft.xp_channel_multipliers = ch?.config_value ?? "";
    multipliersDraft.xp_role_multipliers = ro?.config_value ?? "";
  } catch (e) {
    console.error("Erreur chargement multiplicateurs:", e);
  }
}

/** Valide chaque ligne au format `id:multiplicateur` (id numerique, mult > 0). */
function validateMultipliers(raw: string): { ok: boolean; firstError?: string } {
  const lines = raw.split(/\r?\n/).map((l) => l.trim()).filter(Boolean);
  for (const line of lines) {
    const [idPart, multPart] = line.split(":").map((s) => s?.trim() ?? "");
    if (!/^\d+$/.test(idPart)) return { ok: false, firstError: `ID invalide : "${line}"` };
    const m = Number(multPart);
    if (!Number.isFinite(m) || m <= 0) return { ok: false, firstError: `Multiplicateur invalide : "${line}"` };
  }
  return { ok: true };
}

async function saveMultipliers() {
  if (!guildIdFilter.value) return;
  const checks = [
    validateMultipliers(multipliersDraft.xp_channel_multipliers),
    validateMultipliers(multipliersDraft.xp_role_multipliers),
  ];
  for (const c of checks) {
    if (!c.ok) {
      showError(c.firstError ?? "Format invalide");
      return;
    }
  }
  savingMultipliers.value = true;
  try {
    if (multipliersDraft.xp_channel_multipliers.trim()) {
      await botConfigService.set(
        guildIdFilter.value, PROGRESSION_BOT, "xp_channel_multipliers",
        multipliersDraft.xp_channel_multipliers.trim(),
      );
    } else {
      await botConfigService.remove(guildIdFilter.value, PROGRESSION_BOT, "xp_channel_multipliers");
    }
    if (multipliersDraft.xp_role_multipliers.trim()) {
      await botConfigService.set(
        guildIdFilter.value, PROGRESSION_BOT, "xp_role_multipliers",
        multipliersDraft.xp_role_multipliers.trim(),
      );
    } else {
      await botConfigService.remove(guildIdFilter.value, PROGRESSION_BOT, "xp_role_multipliers");
    }
    success("Multiplicateurs XP enregistrés.");
  } catch (e) {
    console.error(e);
    showError("Erreur sauvegarde multiplicateurs.");
  } finally {
    savingMultipliers.value = false;
  }
}

const xpDraft = reactive<AddXpPayload>({
  guild_id: "",
  user_id: "",
  username: "",
  amount: 100,
  source: "text",
});
const grantingXp = ref(false);

// ── Conduct ─────────────────────────────────────────────
const conductCfg = ref<ConductConfig | null>(null);
const conductDraft = reactive({
  max_points: 12,
  regen_amount: 1,
  regen_interval: "P1D",
  penalty_warn: 1,
  penalty_delete: 1,
  penalty_mute: 3,
  penalty_ban: 6,
});
const savingConduct = ref(false);
const syncingBans = ref(false);
const tickRunning = ref(false);

async function fetchLevelsCfg() {
  if (!guildIdFilter.value) return;
  try {
    levelsCfg.value = await levelsService.getConfig(guildIdFilter.value);
    Object.assign(levelsDraft, {
      xp_per_message: levelsCfg.value.xp_per_message,
      xp_per_voice_minute: levelsCfg.value.xp_per_voice_minute,
      xp_cooldown_secs: levelsCfg.value.xp_cooldown_secs,
      level_up_channel_id: levelsCfg.value.level_up_channel_id ?? "",
      level_up_message: levelsCfg.value.level_up_message,
      excluded_channels: (levelsCfg.value.excluded_channels ?? []).join(","),
      enabled: levelsCfg.value.enabled,
    });
  } catch (e) {
    console.error(e);
    showError("Erreur chargement config niveaux.");
  }
}

async function fetchConductCfg() {
  if (!guildIdFilter.value) return;
  try {
    conductCfg.value = await conductService.getConfig(guildIdFilter.value);
    Object.assign(conductDraft, conductCfg.value);
  } catch (e) {
    console.error(e);
    showError("Erreur chargement config conduite.");
  }
}

async function saveLevels() {
  if (!guildIdFilter.value) return;
  savingLevels.value = true;
  try {
    levelsCfg.value = await levelsService.saveConfig({
      guild_id: guildIdFilter.value,
      xp_per_message: levelsDraft.xp_per_message,
      xp_per_voice_minute: levelsDraft.xp_per_voice_minute,
      xp_cooldown_secs: levelsDraft.xp_cooldown_secs,
      level_up_channel_id: levelsDraft.level_up_channel_id || null,
      level_up_message: levelsDraft.level_up_message,
      excluded_channels: levelsDraft.excluded_channels
        .split(",")
        .map((s) => s.trim())
        .filter(Boolean),
      enabled: levelsDraft.enabled,
    });
    success("Config niveaux enregistrée.");
  } catch (e) {
    console.error(e);
    showError("Erreur sauvegarde config niveaux.");
  } finally {
    savingLevels.value = false;
  }
}

async function grantXp() {
  if (!guildIdFilter.value || !xpDraft.user_id.trim() || xpDraft.amount === 0) {
    showError("user_id et montant requis.");
    return;
  }
  grantingXp.value = true;
  try {
    await levelsService.addXp({
      guild_id: guildIdFilter.value,
      user_id: xpDraft.user_id.trim(),
      username: xpDraft.username.trim() || xpDraft.user_id.trim(),
      amount: xpDraft.amount,
      source: xpDraft.source,
    });
    success(`${xpDraft.amount > 0 ? "+" : ""}${xpDraft.amount} XP attribués.`);
    xpDraft.user_id = "";
    xpDraft.username = "";
    xpDraft.amount = 100;
  } catch (e) {
    console.error(e);
    showError("Erreur attribution XP.");
  } finally {
    grantingXp.value = false;
  }
}

async function saveConduct() {
  if (!guildIdFilter.value) return;
  savingConduct.value = true;
  try {
    conductCfg.value = await conductService.saveConfig({
      guild_id: guildIdFilter.value,
      ...conductDraft,
    });
    success("Config conduite enregistrée.");
  } catch (e) {
    console.error(e);
    showError("Erreur sauvegarde config conduite.");
  } finally {
    savingConduct.value = false;
  }
}

async function runRegenTick() {
  tickRunning.value = true;
  try {
    await conductService.runRegenTick();
    success("Tick de régénération exécuté.");
  } catch (e) {
    console.error(e);
    showError("Erreur lors du tick.");
  } finally {
    tickRunning.value = false;
  }
}

async function syncBans() {
  syncingBans.value = true;
  try {
    const r = await conductService.syncBanProposals();
    success(`${r.created} proposition(s) de ban créée(s).`);
  } catch (e) {
    console.error(e);
    showError("Erreur sync ban proposals.");
  } finally {
    syncingBans.value = false;
  }
}

onMounted(() => {
  fetchLevelsCfg();
  fetchConductCfg();
  fetchMultipliers();
});
watch(guildIdFilter, () => {
  fetchLevelsCfg();
  fetchConductCfg();
  fetchMultipliers();
});
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>⚙️ Levels & Conduite — Configuration</h1>
      <p class="lede">
        Paramètres XP / niveau (texte + vocal) et système de conduite
        (points, regen, escalade automatique vers ban à 0 point).
      </p>
    </header>

    <div class="grid">
      <!-- ── Levels config ── -->
      <section class="card">
        <h2>📈 Niveaux & XP</h2>
        <form @submit.prevent="saveLevels" class="form">
          <label class="toggle full">
            <input v-model="levelsDraft.enabled" type="checkbox" />
            Système actif
          </label>
          <label>
            XP par message
            <input v-model.number="levelsDraft.xp_per_message" type="number" min="0" />
          </label>
          <label>
            XP par minute vocale
            <input v-model.number="levelsDraft.xp_per_voice_minute" type="number" min="0" />
          </label>
          <label>
            Cooldown XP (s)
            <input v-model.number="levelsDraft.xp_cooldown_secs" type="number" min="0" />
          </label>
          <label>
            Salon level-up (ID)
            <input v-model="levelsDraft.level_up_channel_id" placeholder="vide = pas d'annonce" />
          </label>
          <label class="full">
            Message level-up (variables {user}, {level})
            <input v-model="levelsDraft.level_up_message" />
          </label>
          <label class="full">
            Salons exclus (IDs séparés par virgules)
            <input v-model="levelsDraft.excluded_channels" placeholder="ID1,ID2,..." />
          </label>
          <div class="actions full">
            <button type="submit" class="btn-primary" :disabled="savingLevels">
              {{ savingLevels ? "Enregistrement…" : "Enregistrer" }}
            </button>
          </div>
        </form>
      </section>

      <!-- ── Multiplicateurs XP ── -->
      <section class="card">
        <h2>✖️ Multiplicateurs XP</h2>
        <p class="hint">
          Ajuste l'XP gagné dans certains salons ou par les utilisateurs portant
          certains rôles. Format : <code>ID:multiplicateur</code> par ligne.
          <strong>2.0</strong> = double XP, <strong>0.5</strong> = moitié.
          Appliqué à la fois sur le texte et le vocal.
        </p>
        <form @submit.prevent="saveMultipliers" class="form">
          <label class="full">
            Multiplicateurs par <strong>salon</strong>
            <textarea
              v-model="multipliersDraft.xp_channel_multipliers"
              rows="4"
              placeholder="123456789012345678:2.0
987654321098765432:0.5"
            ></textarea>
          </label>
          <label class="full">
            Multiplicateurs par <strong>rôle</strong>
            <textarea
              v-model="multipliersDraft.xp_role_multipliers"
              rows="4"
              placeholder="111222333444555666:2.0   (ex: rôle VIP)"
            ></textarea>
          </label>
          <p class="hint">
            <strong>Cumul</strong> : si un user VIP (rôle ×2) écrit dans un salon ×0.5, il gagne
            <code>base × 0.5 × 2 = base</code> (XP normal). Combinaisons multiplicatives.
          </p>
          <div class="actions full">
            <button type="submit" class="btn-primary" :disabled="savingMultipliers">
              {{ savingMultipliers ? "Enregistrement…" : "Enregistrer les multiplicateurs" }}
            </button>
          </div>
        </form>
      </section>

      <!-- ── Add XP manuel ── -->
      <section class="card">
        <h2>🎁 Attribuer XP manuel</h2>
        <p class="hint">
          Ajoute (ou retire si négatif) des points XP à un utilisateur.
          Permet de corriger un farming abusif ou récompenser manuellement.
        </p>
        <form @submit.prevent="grantXp" class="form">
          <label>
            User ID *
            <input v-model="xpDraft.user_id" required />
          </label>
          <label>
            Username
            <input v-model="xpDraft.username" placeholder="(optionnel)" />
          </label>
          <label>
            Montant *
            <input v-model.number="xpDraft.amount" type="number" required />
          </label>
          <label>
            Source
            <select v-model="xpDraft.source">
              <option value="text">Texte</option>
              <option value="voice">Vocal</option>
            </select>
          </label>
          <div class="actions full">
            <button type="submit" class="btn-primary" :disabled="grantingXp">
              {{ grantingXp ? "…" : "Attribuer" }}
            </button>
          </div>
        </form>
      </section>

      <!-- ── Conduct config ── -->
      <section class="card">
        <h2>🛡️ Système de conduite</h2>
        <form @submit.prevent="saveConduct" class="form">
          <label>
            Points max
            <input v-model.number="conductDraft.max_points" type="number" min="1" />
          </label>
          <label>
            Regen (points par tick)
            <input v-model.number="conductDraft.regen_amount" type="number" min="0" />
          </label>
          <label>
            Intervalle regen (ISO 8601)
            <input v-model="conductDraft.regen_interval" placeholder="P1D" />
          </label>
          <label>
            Pénalité warn
            <input v-model.number="conductDraft.penalty_warn" type="number" min="0" />
          </label>
          <label>
            Pénalité delete
            <input v-model.number="conductDraft.penalty_delete" type="number" min="0" />
          </label>
          <label>
            Pénalité mute
            <input v-model.number="conductDraft.penalty_mute" type="number" min="0" />
          </label>
          <label>
            Pénalité ban
            <input v-model.number="conductDraft.penalty_ban" type="number" min="0" />
          </label>
          <div class="actions full">
            <button type="submit" class="btn-primary" :disabled="savingConduct">
              {{ savingConduct ? "Enregistrement…" : "Enregistrer" }}
            </button>
          </div>
        </form>
      </section>

      <!-- ── Conduct actions ── -->
      <section class="card">
        <h2>⚡ Actions conduite</h2>
        <p class="hint">
          Le worker exécute ces tâches périodiquement. Les boutons ci-dessous
          permettent de forcer manuellement (debug / déblocage).
        </p>
        <div class="action-buttons">
          <button class="btn-secondary" @click="runRegenTick" :disabled="tickRunning">
            {{ tickRunning ? "…" : "Forcer le tick de régénération" }}
          </button>
          <button class="btn-warn" @click="syncBans" :disabled="syncingBans">
            {{ syncingBans ? "…" : "Sync ban proposals manuel" }}
          </button>
        </div>
      </section>
    </div>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 16px;
}
.form {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 12px;
}
.form label {
  display: flex;
  flex-direction: column;
  gap: 4px;
  font-size: 0.9rem;
}
.form label.full {
  grid-column: span 2;
}
.form input,
.form select {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
  font-family: inherit;
}
.toggle {
  flex-direction: row !important;
  align-items: center;
  gap: 8px;
  cursor: pointer;
}
.action-buttons {
  display: flex;
  flex-direction: column;
  gap: 8px;
}
.hint {
  font-size: 0.85rem;
  color: var(--text-secondary);
  margin-bottom: 12px;
}
</style>
