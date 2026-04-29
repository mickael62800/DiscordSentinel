<script setup lang="ts">
import { onMounted } from "vue";
import { useRoute, useRouter } from "vue-router";
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

function onGuildChange(event: Event) {
  const value = (event.target as HTMLSelectElement).value;
  selectGuild(value === "" ? null : value);
}

async function handleLogout() {
  await logout();
  router.push("/login");
}

function goHome() {
  if (route.path !== "/") router.push("/");
}

onMounted(() => {
  fetchGuilds();
});
</script>

<template>
  <header class="topbar">
    <button class="brand" type="button" title="Accueil" @click="goHome">
      <img src="/logo.png" alt="Sentinel" class="logo-icon" />
      <span class="logo-text">Sentinel</span>
    </button>

    <div class="spacer" />

    <div class="guild-selector">
      <select
        class="guild-select"
        :value="selectedGuildId ?? ''"
        @change="onGuildChange"
      >
        <option value="">Tous les serveurs</option>
        <option v-for="g in guilds" :key="g.guild_id" :value="g.guild_id">
          {{ g.name }}
        </option>
      </select>
    </div>

    <div class="status-indicator" :title="wsConnected ? 'Connecte' : 'Deconnecte'">
      <StatusDot :status="wsConnected ? 'online' : 'offline'" />
    </div>

    <button class="bell-btn" title="Notifications" @click="togglePanel">
      <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
        <path d="M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9" />
        <path d="M13.73 21a2 2 0 01-3.46 0" />
      </svg>
      <span v-if="unreadCount > 0" class="bell-badge">{{ unreadCount }}</span>
    </button>

    <div v-if="user" class="user-block">
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

    <NotificationPanel v-if="panelOpen" />
  </header>
</template>

<style scoped>
.topbar {
  position: relative;
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 10px 20px;
  background-color: var(--bg-secondary);
  border-bottom: 1px solid var(--border);
  user-select: none;
  flex-shrink: 0;
}

.brand {
  display: flex;
  align-items: center;
  gap: 10px;
  background: none;
  padding: 4px 6px;
  border-radius: 8px;
}
.brand:hover {
  background-color: var(--bg-hover);
}

.logo-icon {
  width: 32px;
  height: 32px;
  border-radius: 8px;
  object-fit: contain;
}

.logo-text {
  font-size: 16px;
  font-weight: 700;
  color: var(--text-primary);
}

.spacer {
  flex: 1;
}

.guild-select {
  padding: 7px 28px 7px 10px;
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
  min-width: 180px;
}
.guild-select:hover {
  border-color: var(--accent);
}
.guild-select:focus {
  outline: none;
  border-color: var(--accent);
  box-shadow: var(--focus-ring);
}

.status-indicator {
  display: flex;
  align-items: center;
  padding: 0 4px;
}

.bell-btn {
  position: relative;
  width: 34px;
  height: 34px;
  padding: 7px;
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

.user-block {
  display: flex;
  align-items: center;
  gap: 8px;
  padding-left: 12px;
  margin-left: 4px;
  border-left: 1px solid var(--border);
}

.user-avatar {
  width: 32px;
  height: 32px;
  border-radius: 50%;
  flex-shrink: 0;
}

.user-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
  max-width: 140px;
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
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.logout-btn {
  width: 30px;
  height: 30px;
  padding: 5px;
  background: none;
  border-radius: 6px;
  color: var(--text-secondary);
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

@media (max-width: 700px) {
  .user-info {
    display: none;
  }
  .logo-text {
    display: none;
  }
  .topbar {
    padding: 10px 12px;
  }
}
</style>
