<script setup lang="ts">
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import NavItem from "../molecules/NavItem.vue";
import StatusDot from "../atoms/StatusDot.vue";
import NotificationPanel from "./NotificationPanel.vue";
import { useAuth } from "../../composables/useAuth";
import { useNotifications } from "../../composables/useNotifications";
import { useRealtime } from "../../composables/useRealtime";
import { useGuildSelector } from "../../composables/useGuildSelector";

const route = useRoute();
const router = useRouter();
const { user, logout, avatarUrl } = useAuth();
const { unreadCount, panelOpen, togglePanel } = useNotifications();
const { connected: wsConnected } = useRealtime();
const { guilds, selectedGuildId, fetchGuilds, selectGuild } = useGuildSelector();

// ── Sidebar groups (collapsable) ─────────────────────────
// L état (collapsed / expanded) est persiste en localStorage. Le groupe
// contenant la route active est auto-expanded.
const STORAGE_KEY = "sidebar.collapsed.v1";
const collapsed = ref<Record<string, boolean>>(loadCollapsed());

function loadCollapsed(): Record<string, boolean> {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}
function saveCollapsed() {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(collapsed.value));
  } catch {
    /* ignore */
  }
}
function toggleGroup(key: string) {
  collapsed.value[key] = !collapsed.value[key];
  saveCollapsed();
}

watch(collapsed, saveCollapsed, { deep: true });

const generalItems = [
  { path: "/", label: "Tableau de bord", icon: "grid" },
];

const moderationItems = [
  { path: "/moderation", label: "Moderation", icon: "gavel" },
  { path: "/members", label: "Membres", icon: "users" },
  { path: "/rules", label: "Regles", icon: "shield" },
  { path: "/strikes", label: "Strikes", icon: "alert-triangle" },
  { path: "/notes", label: "Notes", icon: "edit-3" },
  { path: "/reminders", label: "Reminders", icon: "clock" },
  { path: "/evidence", label: "Preuves", icon: "paperclip" },
  { path: "/review", label: "Reviews", icon: "check-circle" },
  { path: "/modstats", label: "Modstats", icon: "bar-chart-2" },
  { path: "/name-history", label: "Historique pseudos", icon: "user-x" },
];

const communityItems = [
  { path: "/welcome", label: "Bienvenue", icon: "user-plus" },
  { path: "/tickets", label: "Tickets", icon: "ticket" },
  { path: "/voice-channels", label: "Vocaux", icon: "mic" },
  { path: "/voice-themes", label: "Themes vocaux", icon: "layers" },
  { path: "/role-panels", label: "Roles", icon: "users" },
  { path: "/levels", label: "Niveaux", icon: "trending-up" },
  { path: "/levels-config", label: "Niveaux config", icon: "sliders" },
  { path: "/sponsorships", label: "Parrainages", icon: "user-check" },
  { path: "/temp-roles", label: "Roles temp.", icon: "clock" },
];

const securityItems = [
  { path: "/security", label: "Securite", icon: "zap" },
  { path: "/automod", label: "Automod", icon: "shield" },
  { path: "/audit", label: "Audit", icon: "clipboard" },
];

const logItems = [
  { path: "/logs", label: "Journaux", icon: "list" },
];

const gameItems = [
  { path: "/games", label: "Jeux", icon: "layers" },
  { path: "/coude", label: "Coup de Coude", icon: "zap" },
  { path: "/coude/social", label: "Coude social", icon: "users" },
  { path: "/blackjack", label: "Blackjack", icon: "layers" },
  { path: "/slot", label: "Slot machine", icon: "dollar-sign" },
  { path: "/wheel", label: "Roue du Destin", icon: "refresh-cw" },
  { path: "/wallet", label: "Wallet", icon: "dollar-sign" },
  { path: "/tournaments", label: "Tournoi hebdo", icon: "zap" },
  { path: "/taunts", label: "Railleries", icon: "zap" },
];

const configItems = [
  { path: "/component-config", label: "Composants", icon: "cpu" },
  { path: "/rbac", label: "Acces RBAC", icon: "shield" },
  { path: "/system/operations", label: "System ops", icon: "activity" },
  { path: "/settings", label: "Parametres", icon: "settings" },
];

// Définition des groupes collapsables (clé persistée en localStorage).
const groups = [
  { key: "moderation", label: "Moderation", items: moderationItems },
  { key: "community", label: "Communaute", items: communityItems },
  { key: "security", label: "Securite", items: securityItems },
  { key: "logs", label: "Logs", items: logItems },
  { key: "games", label: "Jeu", items: gameItems },
  { key: "config", label: "Configuration", items: configItems },
];

// Détecte le groupe contenant la route courante → toujours expanded.
const activeGroupKey = computed(() => {
  for (const g of groups) {
    if (g.items.some((i) => route.path === i.path)) return g.key;
  }
  return null;
});

function isExpanded(key: string): boolean {
  // Le groupe actif force l affichage, peu importe collapsed.
  if (activeGroupKey.value === key) return true;
  // Defaut : expanded sauf si l user a explicitement collapsed.
  return collapsed.value[key] !== true;
}

function onGuildChange(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  selectGuild(value === "" ? null : value);
}

async function handleLogout() {
  await logout();
  router.push("/login");
}

onMounted(() => {
  fetchGuilds();
});
</script>

<template>
  <aside class="sidebar">
    <div class="sidebar-header">
      <div class="logo">
        <span class="logo-icon">S</span>
        <span class="logo-text">Sentinel</span>
      </div>
    </div>

    <!-- Selecteur de serveur -->
    <div class="guild-selector">
      <label class="guild-label">Serveur</label>
      <select
        class="guild-select"
        :value="selectedGuildId ?? ''"
        @change="onGuildChange"
      >
        <option value="">Tous les serveurs</option>
        <option
          v-for="g in guilds"
          :key="g.guild_id"
          :value="g.guild_id"
        >
          {{ g.name }}
        </option>
      </select>
    </div>

    <nav class="sidebar-nav">
      <NavItem
        v-for="item in generalItems"
        :key="item.path"
        :path="item.path"
        :label="item.label"
        :icon="item.icon"
        :active="route.path === item.path"
      />

      <template v-for="group in groups" :key="group.key">
        <button
          class="nav-group-header"
          :class="{ collapsed: !isExpanded(group.key) }"
          type="button"
          @click="toggleGroup(group.key)"
        >
          <svg class="chevron" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="6 9 12 15 18 9" />
          </svg>
          <span>{{ group.label }}</span>
          <span class="group-count">{{ group.items.length }}</span>
        </button>
        <div v-if="isExpanded(group.key)" class="nav-group-items">
          <NavItem
            v-for="item in group.items"
            :key="item.path"
            :path="item.path"
            :label="item.label"
            :icon="item.icon"
            :active="route.path === item.path"
          />
        </div>
      </template>
    </nav>

    <!-- Info utilisateur -->
    <div v-if="user" class="sidebar-user">
      <img :src="avatarUrl(user)" :alt="user.username" class="user-avatar" />
      <div class="user-info">
        <span class="user-name">{{ user.global_name ?? user.username }}</span>
        <span class="user-tag">{{ user.username }}</span>
      </div>
      <button class="logout-btn" title="Deconnexion" @click="handleLogout">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M9 21H5a2 2 0 01-2-2V5a2 2 0 012-2h4" />
          <polyline points="16 17 21 12 16 7" />
          <line x1="21" y1="12" x2="9" y2="12" />
        </svg>
      </button>
    </div>

    <div class="sidebar-footer">
      <div class="footer-row">
        <div class="status-indicator">
          <StatusDot :status="wsConnected ? 'online' : 'offline'" />
          <span class="status-text">{{ wsConnected ? "Connecte" : "Deconnecte" }}</span>
        </div>

        <!-- Cloche notifications -->
        <button class="bell-btn" title="Notifications" @click="togglePanel">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9" />
            <path d="M13.73 21a2 2 0 01-3.46 0" />
          </svg>
          <span v-if="unreadCount > 0" class="bell-badge">{{ unreadCount }}</span>
        </button>
      </div>
    </div>

    <!-- Panneau de notifications -->
    <NotificationPanel v-if="panelOpen" />
  </aside>
</template>

<style scoped>
.sidebar {
  width: var(--sidebar-width);
  min-width: var(--sidebar-width);
  background-color: var(--bg-secondary);
  display: flex;
  flex-direction: column;
  border-right: 1px solid var(--border);
  user-select: none;
  position: relative;
}

.sidebar-header {
  padding: 20px;
  border-bottom: 1px solid var(--border);
}

.logo {
  display: flex;
  align-items: center;
  gap: 10px;
}

.logo-icon {
  width: 36px;
  height: 36px;
  background: linear-gradient(135deg, var(--accent), var(--accent-alt));
  border-radius: 10px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 18px;
  color: white;
}

.logo-text {
  font-size: 18px;
  font-weight: 700;
  color: var(--text-primary);
}

/* Selecteur de serveur */
.guild-selector {
  padding: 12px 16px;
  border-bottom: 1px solid var(--border);
}

.guild-label {
  display: block;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.5px;
  color: var(--text-secondary);
  margin-bottom: 6px;
}

.guild-select {
  width: 100%;
  padding: 8px 10px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-primary);
  color: var(--text-primary);
  font-size: 13px;
  cursor: pointer;
  appearance: none;
  background-image: url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' width='12' height='12' viewBox='0 0 24 24' fill='none' stroke='%23888' stroke-width='2'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E");
  background-repeat: no-repeat;
  background-position: right 8px center;
  padding-right: 28px;
}

.guild-select:hover {
  border-color: var(--accent);
}

.guild-select:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
}

.sidebar-nav {
  flex: 1;
  padding: 12px 8px;
  display: flex;
  flex-direction: column;
  gap: 2px;
  overflow-y: auto;
}

.nav-separator {
  display: flex;
  align-items: center;
  gap: 8px;
  margin: 12px 14px 6px;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.8px;
  color: var(--text-secondary);
  opacity: 0.6;
}

.nav-separator::after {
  content: "";
  flex: 1;
  height: 1px;
  background-color: var(--border);
}

/* Groupe collapsable (header cliquable) */
.nav-group-header {
  display: flex;
  align-items: center;
  gap: 6px;
  width: calc(100% - 16px);
  margin: 12px 8px 4px;
  padding: 4px 8px;
  background: none;
  border: none;
  border-radius: 6px;
  font-size: 10px;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.8px;
  color: var(--text-secondary);
  cursor: pointer;
  text-align: left;
  transition: background-color var(--transition-fast, 0.15s);
}
.nav-group-header:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}
.nav-group-header .chevron {
  width: 12px;
  height: 12px;
  flex-shrink: 0;
  transition: transform 0.2s ease;
}
.nav-group-header.collapsed .chevron {
  transform: rotate(-90deg);
}
.nav-group-header span:not(.group-count) {
  flex: 1;
}
.nav-group-header .group-count {
  font-size: 9px;
  background: var(--bg-primary);
  padding: 1px 6px;
  border-radius: 8px;
  color: var(--text-secondary);
  opacity: 0.7;
}
.nav-group-items {
  display: flex;
  flex-direction: column;
  gap: 2px;
}

/* Section utilisateur */
.sidebar-user {
  display: flex;
  align-items: center;
  gap: 10px;
  padding: 12px 16px;
  border-top: 1px solid var(--border);
}

.user-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  flex-shrink: 0;
}

.user-info {
  flex: 1;
  min-width: 0;
  display: flex;
  flex-direction: column;
}

.user-name {
  font-size: 13px;
  font-weight: 600;
  color: var(--text-primary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.user-tag {
  font-size: 11px;
  color: var(--text-secondary);
}

.logout-btn {
  width: 28px;
  height: 28px;
  padding: 4px;
  background: none;
  border-radius: 6px;
  color: var(--text-secondary);
  flex-shrink: 0;
  display: flex;
  align-items: center;
  justify-content: center;
}

.logout-btn:hover {
  background-color: var(--bg-hover);
  color: var(--danger);
}

.logout-btn svg {
  width: 16px;
  height: 16px;
}

.sidebar-footer {
  padding: 12px 20px;
  border-top: 1px solid var(--border);
}

.footer-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
}

.status-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
}

.status-text {
  font-size: 12px;
  color: var(--text-secondary);
}

/* Cloche */
.bell-btn {
  position: relative;
  width: 32px;
  height: 32px;
  padding: 6px;
  background: none;
  border-radius: 8px;
  color: var(--text-secondary);
  display: flex;
  align-items: center;
  justify-content: center;
}

.bell-btn:hover {
  background-color: var(--bg-hover);
  color: var(--text-primary);
}

.bell-btn svg {
  width: 18px;
  height: 18px;
}

.bell-badge {
  position: absolute;
  top: 2px;
  right: 2px;
  min-width: 16px;
  height: 16px;
  border-radius: 8px;
  background-color: var(--danger);
  color: white;
  font-size: 10px;
  font-weight: 700;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0 4px;
}
</style>
