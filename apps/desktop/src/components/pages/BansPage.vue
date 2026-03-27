<script setup lang="ts">
import { computed } from "vue";
import { useBans } from "../../composables/useBans";
import type { SelectOption } from "../../types";
import AppBadge from "../atoms/AppBadge.vue";
import AppSelect from "../atoms/AppSelect.vue";

const { filteredBans, servers, totalBans, loading, searchQuery, filterServer } = useBans();

const serverOptions = computed<SelectOption[]>(() => [
  { value: "all", label: "Tous les serveurs" },
  ...servers.value.map((s) => ({ value: s, label: s })),
]);
</script>

<template>
  <div class="bans">
    <div class="bans-header">
      <h1>Comptes bannis</h1>
      <span class="ban-count">{{ totalBans }} au total</span>
    </div>

    <div class="filters">
      <input
        v-model="searchQuery"
        type="text"
        placeholder="Rechercher par nom, ID ou raison..."
        class="search-input"
      />
      <AppSelect v-model="filterServer" :options="serverOptions" />
    </div>

    <div v-if="loading" class="loading">Chargement...</div>

    <div v-else class="ban-list">
      <div v-for="ban in filteredBans" :key="ban.id" class="ban-card">
        <div class="ban-user">
          <div class="user-avatar-placeholder">{{ ban.username.charAt(0).toUpperCase() }}</div>
          <div class="user-info">
            <span class="username">{{ ban.username }}</span>
            <span class="user-id">{{ ban.user_id }}</span>
          </div>
        </div>

        <div class="ban-details">
          <div class="detail-row">
            <span class="detail-label">Serveur</span>
            <span>{{ ban.server }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Raison</span>
            <span class="reason">{{ ban.reason }}</span>
          </div>
          <div class="detail-row">
            <span class="detail-label">Banni par</span>
            <AppBadge :label="ban.moderator" variant="info" />
          </div>
          <div class="detail-row">
            <span class="detail-label">Date</span>
            <span class="mono">{{ ban.created_at }}</span>
          </div>
        </div>
      </div>

      <div v-if="filteredBans.length === 0" class="empty">
        Aucun compte banni{{ searchQuery ? " correspondant a votre recherche" : "" }}
      </div>
    </div>
  </div>
</template>

<style scoped>
.bans-header {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 24px;
}

.bans-header h1 {
  margin: 0;
}

.ban-count {
  font-size: 13px;
  color: var(--text-secondary);
  background-color: var(--bg-card);
  padding: 4px 10px;
  border-radius: 6px;
  border: 1px solid var(--border);
}

.filters {
  display: flex;
  gap: 12px;
  margin-bottom: 16px;
}

.search-input {
  flex: 1;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  color: var(--text-primary);
  font-size: 14px;
  outline: none;
  transition: border-color 0.2s;
}

.search-input:focus {
  border-color: var(--accent);
}

.search-input::placeholder {
  color: var(--text-secondary);
  opacity: 0.6;
}

.ban-list {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.ban-card {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-left: 3px solid var(--danger);
  border-radius: 12px;
  padding: 20px;
  display: flex;
  gap: 24px;
  align-items: flex-start;
}

.ban-user {
  display: flex;
  align-items: center;
  gap: 12px;
  min-width: 200px;
}

.user-avatar-placeholder {
  width: 40px;
  height: 40px;
  border-radius: 50%;
  background: linear-gradient(135deg, var(--danger), #ff6b6b);
  display: flex;
  align-items: center;
  justify-content: center;
  font-weight: 700;
  font-size: 16px;
  color: white;
  flex-shrink: 0;
}

.user-info {
  display: flex;
  flex-direction: column;
  gap: 2px;
  min-width: 0;
}

.username {
  font-weight: 600;
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.user-id {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.ban-details {
  flex: 1;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.detail-row {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 13px;
}

.detail-label {
  color: var(--text-secondary);
  min-width: 80px;
  font-weight: 500;
}

.reason {
  color: var(--text-primary);
}

.empty {
  text-align: center;
  color: var(--text-secondary);
  padding: 40px;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
}
</style>
