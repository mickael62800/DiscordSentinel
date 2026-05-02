<script setup lang="ts">
import { computed, ref, watch } from "vue";
import {
  gamePortalService,
  type PlayerSession,
} from "@/services/gamePortalService";

const props = defineProps<{
  open: boolean;
  serverId: string | null;
  serverName: string;
}>();

const emit = defineEmits<{
  (e: "close"): void;
}>();

const sessions = ref<PlayerSession[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

watch(
  () => [props.open, props.serverId],
  async () => {
    if (props.open && props.serverId) {
      loading.value = true;
      error.value = null;
      try {
        sessions.value = await gamePortalService.listSessions(
          props.serverId,
          200,
          0,
        );
      } catch (e) {
        error.value = e instanceof Error ? e.message : String(e);
        sessions.value = [];
      } finally {
        loading.value = false;
      }
    }
  },
  { immediate: true },
);

const totalSessions = computed(() => sessions.value.length);
const activeSessions = computed(
  () => sessions.value.filter((s) => s.left_at === null).length,
);
const uniquePlayers = computed(
  () => new Set(sessions.value.map((s) => s.player_name)).size,
);
const totalPlaytime = computed(() => {
  const sec = sessions.value.reduce(
    (acc, s) => acc + (s.duration_seconds ?? 0),
    0,
  );
  return formatDuration(sec);
});

function formatDuration(seconds: number | null): string {
  if (seconds === null || seconds === undefined) return "—";
  if (seconds < 60) return `${seconds}s`;
  const m = Math.floor(seconds / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  const mr = m % 60;
  if (h < 24) return `${h}h${mr.toString().padStart(2, "0")}`;
  const d = Math.floor(h / 24);
  return `${d}j ${h % 24}h`;
}

function formatTime(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "2-digit",
    month: "2-digit",
    year: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function liveDuration(s: PlayerSession): string {
  if (s.left_at) return formatDuration(s.duration_seconds);
  const sec = Math.floor(
    (Date.now() - new Date(s.joined_at).getTime()) / 1000,
  );
  return `${formatDuration(sec)} (en ligne)`;
}
</script>

<template>
  <div v-if="open" class="modal-overlay" @click.self="emit('close')">
    <div class="modal">
      <header class="modal-head">
        <div>
          <h2>Sessions joueurs — {{ serverName }}</h2>
          <p class="modal-sub">
            Historique des connexions/déconnexions tracées par le worker
          </p>
        </div>
        <button class="btn-close" @click="emit('close')" aria-label="Fermer">×</button>
      </header>

      <div class="kpi-row">
        <div class="kpi"><span class="kpi-val">{{ totalSessions }}</span><span class="kpi-lbl">sessions</span></div>
        <div class="kpi"><span class="kpi-val">{{ uniquePlayers }}</span><span class="kpi-lbl">joueurs uniques</span></div>
        <div class="kpi"><span class="kpi-val">{{ activeSessions }}</span><span class="kpi-lbl">en ligne</span></div>
        <div class="kpi"><span class="kpi-val">{{ totalPlaytime }}</span><span class="kpi-lbl">temps total</span></div>
      </div>

      <div class="modal-body">
        <div v-if="loading" class="empty">Chargement…</div>
        <div v-else-if="error" class="empty err">⚠ {{ error }}</div>
        <div v-else-if="sessions.length === 0" class="empty">
          Aucune session enregistrée pour ce serveur.
        </div>
        <table v-else class="sessions-table">
          <thead>
            <tr>
              <th>Joueur</th>
              <th>Connexion</th>
              <th>Déconnexion</th>
              <th>Durée</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="s in sessions" :key="s.id" :class="{ active: s.left_at === null }">
              <td>
                <span class="player-dot" :class="{ on: s.left_at === null }" />
                {{ s.player_name }}
              </td>
              <td>{{ formatTime(s.joined_at) }}</td>
              <td>{{ s.left_at ? formatTime(s.left_at) : "—" }}</td>
              <td class="duration">{{ liveDuration(s) }}</td>
            </tr>
          </tbody>
        </table>
      </div>

      <footer class="modal-foot">
        <button class="btn-cancel" @click="emit('close')">Fermer</button>
      </footer>
    </div>
  </div>
</template>

<style scoped>
.modal-overlay {
  position: fixed;
  inset: 0;
  background: var(--modal-overlay, rgba(0, 0, 0, 0.6));
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  padding: 16px;
}

.modal {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg, 12px);
  width: 100%;
  max-width: 800px;
  max-height: 90vh;
  display: flex;
  flex-direction: column;
  box-shadow: var(--shadow-xl);
}

.modal-head {
  display: flex;
  justify-content: space-between;
  padding: var(--space-lg);
  border-bottom: 1px solid var(--border);
}

.modal-head h2 {
  margin: 0;
  font-size: 16px;
}

.modal-sub {
  margin: 4px 0 0;
  font-size: 12px;
  color: var(--text-secondary);
}

.btn-close {
  width: 32px;
  height: 32px;
  border: none;
  background: transparent;
  color: var(--text-secondary);
  font-size: 22px;
  cursor: pointer;
  border-radius: var(--radius-sm);
}

.btn-close:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

.kpi-row {
  display: grid;
  grid-template-columns: repeat(4, 1fr);
  gap: 8px;
  padding: var(--space-md) var(--space-lg) 0;
}

.kpi {
  display: flex;
  flex-direction: column;
  align-items: center;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 8px 12px;
}

.kpi-val {
  font-weight: 700;
  font-size: 16px;
  color: var(--text-primary);
}

.kpi-lbl {
  font-size: 10px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.04em;
  margin-top: 2px;
}

.modal-body {
  padding: var(--space-md) var(--space-lg);
  overflow-y: auto;
  flex: 1;
  min-height: 0;
}

.empty {
  text-align: center;
  padding: var(--space-2xl);
  color: var(--text-secondary);
}

.empty.err {
  color: var(--danger);
}

.sessions-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

.sessions-table th {
  text-align: left;
  padding: 8px;
  font-weight: 600;
  font-size: 11px;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  color: var(--text-secondary);
  border-bottom: 1px solid var(--border);
  position: sticky;
  top: 0;
  background: var(--bg-secondary);
}

.sessions-table td {
  padding: 8px;
  border-bottom: 1px solid color-mix(in srgb, var(--border) 40%, transparent);
  color: var(--text-primary);
}

.sessions-table tr.active td {
  background: color-mix(in srgb, var(--success) 6%, transparent);
}

.player-dot {
  display: inline-block;
  width: 7px;
  height: 7px;
  border-radius: 50%;
  background: var(--text-secondary);
  margin-right: 6px;
}

.player-dot.on {
  background: var(--success);
  box-shadow: 0 0 6px var(--success);
}

.duration {
  font-family: monospace;
  color: var(--text-secondary);
}

.modal-foot {
  padding: var(--space-md) var(--space-lg);
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: flex-end;
}

.btn-cancel {
  padding: 8px 18px;
  border-radius: 6px;
  font-size: 13px;
  font-weight: 600;
  cursor: pointer;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text-secondary);
}

.btn-cancel:hover {
  background: var(--bg-hover);
  color: var(--text-primary);
}

@media (max-width: 640px) {
  .kpi-row {
    grid-template-columns: repeat(2, 1fr);
  }
  .sessions-table {
    font-size: 11px;
  }
  .sessions-table th,
  .sessions-table td {
    padding: 6px 4px;
  }
}
</style>
