<script setup lang="ts">
import { onMounted } from "vue";
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

const generalItems = [
  { path: "/", label: "Tableau de bord", icon: "grid" },
  { path: "/analytics", label: "Analytiques", icon: "trending-up" },
];

const moderationItems = [
  { path: "/moderation", label: "Moderation", icon: "gavel" },
  { path: "/members", label: "Membres", icon: "users" },
  { path: "/rules", label: "Regles", icon: "shield" },
];

const communityItems = [
  { path: "/tickets", label: "Tickets", icon: "ticket" },
  { path: "/voice-channels", label: "Vocaux", icon: "mic" },
  { path: "/role-panels", label: "Roles", icon: "users" },
  { path: "/levels", label: "Niveaux", icon: "trending-up" },
];

const securityItems = [
  { path: "/security", label: "Securite", icon: "zap" },
  { path: "/audit", label: "Audit", icon: "clipboard" },
];

const logItems = [
  { path: "/logs", label: "Journaux", icon: "list" },
];

const gameItems = [
  { path: "/coude", label: "Coup de Coude", icon: "zap" },
];

const aiItems = [
  { path: "/ai-training", label: "Entrainement", icon: "layers" },
  { path: "/ia-config", label: "Config IA", icon: "brain" },
];

const configItems = [
  { path: "/component-config", label: "Composants", icon: "cpu" },
  { path: "/rbac", label: "Acces RBAC", icon: "shield" },
  { path: "/settings", label: "Parametres", icon: "settings" },
];

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

      <div class="nav-separator"><span>Moderation</span></div>
      <NavItem
        v-for="item in moderationItems"
        :key="item.path"
        :path="item.path"
        :label="item.label"
        :icon="item.icon"
        :active="route.path === item.path"
      />

      <div class="nav-separator"><span>Communaute</span></div>
      <NavItem
        v-for="item in communityItems"
        :key="item.path"
        :path="item.path"
        :label="item.label"
        :icon="item.icon"
        :active="route.path === item.path"
      />

      <div class="nav-separator"><span>Securite</span></div>
      <NavItem
        v-for="item in securityItems"
        :key="item.path"
        :path="item.path"
        :label="item.label"
        :icon="item.icon"
        :active="route.path === item.path"
      />

      <div class="nav-separator"><span>Logs</span></div>
      <NavItem
        v-for="item in logItems"
        :key="item.path"
        :path="item.path"
        :label="item.label"
        :icon="item.icon"
        :active="route.path === item.path"
      />

      <div class="nav-separator"><span>Jeu</span></div>
      <NavItem
        v-for="item in gameItems"
        :key="item.path"
        :path="item.path"
        :label="item.label"
        :icon="item.icon"
        :active="route.path === item.path"
      />

      <div class="nav-separator"><span>Intelligence Artificielle</span></div>
      <NavItem
        v-for="item in aiItems"
        :key="item.path"
        :path="item.path"
        :label="item.label"
        :icon="item.icon"
        :active="route.path === item.path"
      />

      <div class="nav-separator"><span>Configuration</span></div>
      <NavItem
        v-for="item in configItems"
        :key="item.path"
        :path="item.path"
        :label="item.label"
        :icon="item.icon"
        :active="route.path === item.path"
      />
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
  background: linear-gradient(135deg, var(--accent), #7c5cfc);
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
  box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
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
