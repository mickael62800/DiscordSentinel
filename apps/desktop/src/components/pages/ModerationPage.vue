<script setup lang="ts">
import { ref } from "vue";
import { useModeration } from "../../composables/useModeration";
import AppButton from "../atoms/AppButton.vue";
import AppBadge from "../atoms/AppBadge.vue";

const { submitting, history, historyLoading, logAction, fetchHistory } = useModeration();

// Action form
const guildId = ref("");
const targetId = ref("");
const targetName = ref("");
const actionType = ref("warn");
const reason = ref("");
const gravity = ref("medium");
const duration = ref<number | undefined>(undefined);
const success = ref<string | null>(null);
const error = ref<string | null>(null);

// History lookup
const lookupGuildId = ref("");
const lookupUserId = ref("");

async function handleSubmit() {
  if (!guildId.value || !targetId.value || !targetName.value || !reason.value) {
    error.value = "L'ID du serveur, l'ID de la cible, le nom de la cible et la raison sont requis.";
    return;
  }
  error.value = null;
  success.value = null;
  try {
    const result = await logAction({
      guildId: guildId.value,
      channelId: "desktop-app",
      moderatorId: "desktop-admin",
      moderatorName: "Desktop Admin",
      targetId: targetId.value,
      targetName: targetName.value,
      actionType: actionType.value,
      reason: reason.value,
      gravity: gravity.value,
      duration: actionType.value === "mute" || actionType.value === "ban" ? duration.value : undefined,
    });
    success.value = `${result.action_type} applique a ${result.target_name}`;
    targetId.value = "";
    targetName.value = "";
    reason.value = "";
    duration.value = undefined;
  } catch (e) {
    error.value = String(e);
  }
}

async function handleLookup() {
  if (!lookupGuildId.value || !lookupUserId.value) return;
  await fetchHistory(lookupGuildId.value, lookupUserId.value);
}

function actionVariant(action: string): "danger" | "warning" | "info" | "default" {
  switch (action) {
    case "ban": return "danger";
    case "mute": return "warning";
    case "warn": return "info";
    default: return "default";
  }
}
</script>

<template>
  <div class="moderation">
    <h1>Moderation</h1>

    <div class="mod-grid">
      <!-- Action form -->
      <div class="mod-card">
        <h2>Appliquer une action</h2>
        <form class="action-form" @submit.prevent="handleSubmit">
          <div class="form-row">
            <div class="field">
              <label>ID du serveur</label>
              <input v-model="guildId" type="text" placeholder="ID du serveur" />
            </div>
          </div>
          <div class="form-row two-col">
            <div class="field">
              <label>ID de l'utilisateur cible</label>
              <input v-model="targetId" type="text" placeholder="ID utilisateur Discord" />
            </div>
            <div class="field">
              <label>Nom de l'utilisateur cible</label>
              <input v-model="targetName" type="text" placeholder="nom#1234" />
            </div>
          </div>
          <div class="form-row two-col">
            <div class="field">
              <label>Action</label>
              <select v-model="actionType">
                <option value="warn">Avertissement</option>
                <option value="mute">Sourdine</option>
                <option value="ban">Bannissement</option>
              </select>
            </div>
            <div class="field">
              <label>Gravite</label>
              <select v-model="gravity">
                <option value="low">Faible</option>
                <option value="medium">Moyen</option>
                <option value="high">Eleve</option>
                <option value="critical">Critique</option>
              </select>
            </div>
          </div>
          <div v-if="actionType === 'mute' || actionType === 'ban'" class="form-row">
            <div class="field">
              <label>Duree (secondes) — laisser vide pour permanent</label>
              <input v-model.number="duration" type="number" placeholder="600 = 10min, 3600 = 1h" :min="0" />
            </div>
          </div>
          <div class="field">
            <label>Raison</label>
            <textarea v-model="reason" rows="2" placeholder="Pourquoi cette action est-elle prise ?"></textarea>
          </div>

          <p v-if="error" class="error-msg">{{ error }}</p>
          <p v-if="success" class="success-msg">{{ success }}</p>

          <AppButton variant="primary" class="submit-btn" :disabled="submitting">
            {{ submitting ? "Application..." : `Appliquer ${actionType}` }}
          </AppButton>
        </form>
      </div>

      <!-- History lookup -->
      <div class="mod-card">
        <h2>Historique de l'utilisateur</h2>
        <div class="lookup-form">
          <div class="form-row two-col">
            <div class="field">
              <label>ID du serveur</label>
              <input v-model="lookupGuildId" type="text" placeholder="ID du serveur" />
            </div>
            <div class="field">
              <label>ID de l'utilisateur</label>
              <input v-model="lookupUserId" type="text" placeholder="ID utilisateur Discord" />
            </div>
          </div>
          <AppButton variant="primary" :disabled="historyLoading" @click="handleLookup">
            {{ historyLoading ? "Chargement..." : "Rechercher" }}
          </AppButton>
        </div>

        <div v-if="history" class="history-result">
          <div class="history-header">
            <span class="history-name">{{ history.target_name }}</span>
            <span class="history-id">{{ history.target_id }}</span>
          </div>
          <div class="history-stats">
            <div class="stat"><span class="stat-num info">{{ history.total_warns }}</span> avertissements</div>
            <div class="stat"><span class="stat-num warning">{{ history.total_mutes }}</span> sourdines</div>
            <div class="stat"><span class="stat-num danger">{{ history.total_bans }}</span> bannissements</div>
          </div>
          <div v-if="history.actions.length > 0" class="history-actions">
            <div v-for="action in history.actions" :key="action.id" class="history-action">
              <AppBadge :label="action.action_type" :variant="actionVariant(action.action_type)" />
              <span class="action-reason">{{ action.reason }}</span>
            </div>
          </div>
          <div v-else class="empty-small">Aucune action enregistree</div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.moderation h1 { margin-bottom: 24px; }

.mod-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
}

@media (max-width: 1100px) {
  .mod-grid { grid-template-columns: 1fr; }
}

.mod-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 24px;
}

.mod-card h2 {
  font-size: 16px;
  font-weight: 600;
  margin-bottom: 20px;
}

.action-form, .lookup-form {
  display: flex;
  flex-direction: column;
  gap: 14px;
}

.form-row { display: flex; gap: 12px; }
.form-row.two-col > .field { flex: 1; }

.field {
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.field label {
  font-size: 12px;
  font-weight: 600;
  color: var(--text-secondary);
}

.field input, .field select, .field textarea {
  width: 100%;
  background-color: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 13px;
  font-family: inherit;
  outline: none;
}

.field input:focus, .field select:focus, .field textarea:focus {
  border-color: var(--accent);
}

.field textarea { resize: vertical; }

.error-msg { color: var(--danger); font-size: 13px; }
.success-msg { color: var(--success); font-size: 13px; }

.submit-btn { width: 100%; margin-top: 4px; }

/* History */
.history-result { margin-top: 20px; }

.history-header {
  display: flex;
  align-items: baseline;
  gap: 8px;
  margin-bottom: 12px;
}

.history-name { font-weight: 600; font-size: 16px; }
.history-id { font-size: 11px; color: var(--text-secondary); font-family: monospace; }

.history-stats {
  display: flex;
  gap: 16px;
  margin-bottom: 16px;
}

.stat {
  font-size: 13px;
  color: var(--text-secondary);
}

.stat-num {
  font-weight: 700;
  font-size: 18px;
  margin-right: 4px;
}

.stat-num.info { color: var(--info); }
.stat-num.warning { color: var(--warning); }
.stat-num.danger { color: var(--danger); }

.history-actions {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.history-action {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 8px 12px;
  background-color: var(--bg-primary);
  border-radius: 8px;
}

.action-reason {
  font-size: 13px;
  color: var(--text-secondary);
}

.empty-small {
  color: var(--text-secondary);
  font-size: 13px;
  text-align: center;
  padding: 16px;
}
</style>
