<script setup lang="ts">
import type { GameServer, GameTemplate } from "@/services/gamePortalService";
import { useComponentVisibility } from "@/composables/useComponentVisibility";

const { visible } = useComponentVisibility();

defineProps<{
  servers: GameServer[];
  templates: GameTemplate[];
  loading: boolean;
  busy: string | null;
  selectedServerId: string | null;
}>();

const emit = defineEmits<{
  select: [id: string];
  toggle: [server: GameServer];
  openConfig: [server: GameServer];
  openSessions: [server: GameServer];
  remove: [server: GameServer];
}>();

function templateById(id: string, templates: GameTemplate[]) {
  return templates.find((t) => t.id === id);
}
function templateBySlug(slug: string | undefined, templates: GameTemplate[]) {
  if (!slug) return undefined;
  return templates.find((t) => t.slug === slug);
}
function templateAccent(slug: string | undefined, templates: GameTemplate[]): string {
  const t = templateBySlug(slug, templates);
  return t?.accent_color ? `#${t.accent_color}` : "var(--accent)";
}
function templateIcon(slug: string | undefined, templates: GameTemplate[]): string {
  return templateBySlug(slug, templates)?.icon ?? "🎮";
}
function formatUptime(server: GameServer): string {
  if (!server.started_at || server.status !== "running") return "—";
  const ms = Date.now() - new Date(server.started_at).getTime();
  const h = Math.floor(ms / 3_600_000);
  const m = Math.floor((ms % 3_600_000) / 60_000);
  if (h >= 24) {
    const d = Math.floor(h / 24);
    return `${d}j ${h % 24}h`;
  }
  return h > 0 ? `${h}h ${m}m` : `${m}m`;
}
</script>

<template>
  <section class="panel servers">
    <div class="panel-head">
      <h2>Serveurs actifs</h2>
      <span class="hint">{{ servers.length }} total</span>
    </div>
    <div class="server-list">
      <div
        v-for="s in servers"
        :key="s.id"
        class="server"
        :class="{ active: s.id === selectedServerId }"
        @click="emit('select', s.id)"
      >
        <div
          class="server-icon"
          :style="{ background: templateAccent(templateById(s.template_id, templates)?.slug, templates) }"
        >
          {{ templateIcon(templateById(s.template_id, templates)?.slug, templates) }}
        </div>
        <div class="server-info">
          <div class="server-name">
            {{ s.name }}
            <span class="status" :class="s.status">{{ s.status }}</span>
          </div>
          <div class="server-meta">
            {{ templateById(s.template_id, templates)?.name ?? '?' }} ·
            <template v-if="s.host_port">port {{ s.host_port }} · </template>
            {{ s.last_player_count }} joueur(s) · up {{ formatUptime(s) }}
          </div>
          <div v-if="s.last_error" class="server-error">⚠ {{ s.last_error }}</div>
        </div>
        <div class="server-actions" @click.stop>
          <button
            class="btn-icon"
            :disabled="busy === s.id"
            :title="s.status === 'running' ? 'Arrêter' : 'Démarrer'"
            @click="emit('toggle', s)"
          >
            {{ s.status === 'running' ? '⏹' : '▶' }}
          </button>
          <button
            v-if="visible('game.server.config_edit')"
            class="btn-icon"
            :disabled="busy === s.id"
            title="Configurer"
            @click="emit('openConfig', s)"
          >⚙</button>
          <button
            class="btn-icon"
            title="Sessions joueurs"
            @click="emit('openSessions', s)"
          >👥</button>
          <button
            v-if="visible('game.server.delete')"
            class="btn-icon btn-icon-danger"
            :disabled="busy === s.id"
            title="Supprimer"
            @click="emit('remove', s)"
          >🗑</button>
        </div>
      </div>
      <div v-if="!loading && servers.length === 0" class="empty">Aucun serveur lancé</div>
      <div v-if="loading && servers.length === 0" class="empty">Chargement…</div>
    </div>
  </section>
</template>

<style scoped>
.panel {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  padding: var(--space-lg);
  display: flex; flex-direction: column;
  min-height: 0;
}
.panel-head {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: var(--space-md);
  margin-bottom: var(--space-md);
}
.panel-head h2 { margin: 0; font-size: 14px; font-weight: 600; letter-spacing: 0.2px; }
.hint { color: var(--text-secondary); font-size: 12px; font-weight: 400; }

.server-list {
  display: flex;
  flex-direction: column;
  gap: var(--space-sm);
  overflow: auto;
  flex: 1;
  min-height: 0;
}
.server {
  display: flex;
  gap: var(--space-md);
  align-items: center;
  padding: var(--space-md);
  border-radius: var(--radius-md);
  background: var(--bg-card);
  border: 1px solid transparent;
  cursor: pointer;
  transition: var(--transition-fast);
}
.server:hover { border-color: var(--border); background: var(--bg-hover); }
.server.active { border-color: var(--accent); background: var(--accent-bg); }

.server-icon {
  width: 40px; height: 40px;
  border-radius: var(--radius-sm);
  display: grid; place-items: center;
  font-size: 20px; flex-shrink: 0;
  box-shadow: var(--shadow-sm);
}
.server-info { flex: 1; min-width: 0; }
.server-name {
  font-weight: 600; font-size: 13px;
  color: var(--text-primary);
  display: flex; flex-wrap: wrap; align-items: center; gap: 6px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}
.server-meta {
  color: var(--text-secondary);
  font-size: 11px;
  margin-top: 2px;
  word-break: break-word;
}

.status {
  font-size: 9px; padding: 2px 6px;
  border-radius: var(--radius-sm);
  text-transform: uppercase; letter-spacing: 0.5px; font-weight: 700;
  flex-shrink: 0;
}
.status.running { background: var(--success-bg); color: var(--success); }
.status.starting { background: var(--warning-bg); color: var(--warning); }
.status.stopped { background: var(--muted-bg); color: var(--text-secondary); }
.status.error { background: var(--danger-bg); color: var(--danger); }

.server-actions { display: flex; gap: 4px; flex-shrink: 0; }
.btn-icon {
  width: 36px; height: 36px;
  border-radius: var(--radius-sm);
  background: var(--bg-primary); color: var(--text-primary);
  border: 1px solid var(--border);
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
  padding: 0;
  display: inline-flex;
  align-items: center;
  justify-content: center;
  transition: var(--transition-fast);
}
.btn-icon:hover:not(:disabled) { background: var(--accent); border-color: var(--accent); color: #fff; }
.btn-icon:disabled { opacity: 0.4; cursor: not-allowed; }
.btn-icon-danger:hover:not(:disabled) { background: var(--danger); border-color: var(--danger); color: #fff; }

.server-error { font-size: 11px; color: var(--danger); margin-top: 4px; word-break: break-word; }
.empty { color: var(--text-secondary); text-align: center; padding: var(--space-xl); font-size: 13px; }
</style>
