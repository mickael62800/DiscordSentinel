<script setup lang="ts">
import { ref } from "vue";
import { useVoiceChannels, useVoiceChannelDetail } from "../../composables/useVoiceChannels";
import AppBadge from "../atoms/AppBadge.vue";

const { filteredChannels, loading, filterKind, publicCount, privateCount, totalCount } = useVoiceChannels();
const { detail, loading: detailLoading, fetchDetail } = useVoiceChannelDetail();

const selectedId = ref<string | null>(null);

async function selectChannel(channelId: string) {
  selectedId.value = channelId;
  await fetchDetail(channelId);
}

function backToList() {
  selectedId.value = null;
  detail.value = null;
}

function kindVariant(kind: string): "info" | "warning" | "default" {
  switch (kind) {
    case "public": return "info";
    case "private": return "warning";
    default: return "default";
  }
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>Salons vocaux</h1>
      <p class="page-subtitle">Salons vocaux temporaires actifs</p>
    </header>

    <!-- Stats -->
    <div class="stats-row">
      <div class="stat-card">
        <span class="stat-value">{{ totalCount }}</span>
        <span class="stat-label">Total</span>
      </div>
      <div class="stat-card">
        <span class="stat-value">{{ publicCount }}</span>
        <span class="stat-label">Public</span>
      </div>
      <div class="stat-card">
        <span class="stat-value">{{ privateCount }}</span>
        <span class="stat-label">Private</span>
      </div>
    </div>

    <!-- Detail view -->
    <div v-if="selectedId && detail" class="detail-view">
      <button class="back-btn" @click="backToList">&larr; Retour</button>

      <div v-if="detailLoading" class="loading">Chargement...</div>
      <div v-else class="detail-content">
        <h2>{{ detail.channel.channel_name }}</h2>
        <div class="detail-grid">
          <div><strong>Owner:</strong> {{ detail.channel.owner_name }}</div>
          <div><strong>Type:</strong> <AppBadge :variant="kindVariant(detail.channel.kind)">{{ detail.channel.kind }}</AppBadge></div>
          <div><strong>Visibility:</strong> {{ detail.channel.visibility }}</div>
          <div><strong>Locked:</strong> {{ detail.channel.locked ? 'Yes' : 'No' }}</div>
          <div><strong>Queue:</strong> {{ detail.channel.queue_enabled ? 'Enabled' : 'Disabled' }}</div>
          <div v-if="detail.channel.member_limit"><strong>Limit:</strong> {{ detail.channel.member_limit }}</div>
          <div v-if="detail.channel.status"><strong>Status:</strong> {{ detail.channel.status }}</div>
          <div><strong>Created:</strong> {{ new Date(detail.channel.created_at).toLocaleString() }}</div>
        </div>

        <h3 v-if="detail.co_admins.length">Co-admins ({{ detail.co_admins.length }})</h3>
        <ul v-if="detail.co_admins.length" class="admin-list">
          <li v-for="ca in detail.co_admins" :key="ca.id">{{ ca.user_name }}</li>
        </ul>

        <h3 v-if="detail.bans.length">Bans ({{ detail.bans.length }})</h3>
        <div v-if="detail.bans.length" class="bans-table">
          <div v-for="ban in detail.bans" :key="ban.id" class="ban-row">
            <span>{{ ban.user_name }}</span>
            <span>by {{ ban.banned_by }}</span>
            <span v-if="ban.reason">{{ ban.reason }}</span>
            <span v-if="ban.expires_at">Expires: {{ new Date(ban.expires_at).toLocaleString() }}</span>
            <AppBadge v-else variant="danger">Permanent</AppBadge>
          </div>
        </div>
      </div>
    </div>

    <!-- List view -->
    <div v-else>
      <div class="filter-row">
        <select v-model="filterKind" class="filter-select">
          <option value="all">All types</option>
          <option value="public">Public</option>
          <option value="private">Private</option>
        </select>
      </div>

      <div v-if="loading" class="loading">Chargement...</div>
      <div v-else-if="filteredChannels.length === 0" class="empty">Aucun salon vocal temporaire actif</div>
      <table v-else class="data-table">
        <thead>
          <tr>
            <th>Nom</th>
            <th>Proprietaire</th>
            <th>Type</th>
            <th>Visibilite</th>
            <th>Verrouille</th>
            <th>File d'attente</th>
            <th>Creation</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="ch in filteredChannels" :key="ch.id" class="clickable" @click="selectChannel(ch.channel_id)">
            <td>{{ ch.channel_name }}</td>
            <td>{{ ch.owner_name }}</td>
            <td><AppBadge :variant="kindVariant(ch.kind)">{{ ch.kind }}</AppBadge></td>
            <td>{{ ch.visibility }}</td>
            <td>{{ ch.locked ? 'Oui' : 'Non' }}</td>
            <td>{{ ch.queue_enabled ? 'Oui' : 'Non' }}</td>
            <td>{{ new Date(ch.created_at).toLocaleString() }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.page {
  padding: 24px;
}

.page-header {
  margin-bottom: 24px;
}

.page-header h1 {
  font-size: 24px;
  font-weight: 700;
  color: var(--text-primary);
}

.page-subtitle {
  color: var(--text-secondary);
  font-size: 14px;
  margin-top: 4px;
}

.stats-row {
  display: flex;
  gap: 16px;
  margin-bottom: 24px;
}

.stat-card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px 24px;
  display: flex;
  flex-direction: column;
  gap: 4px;
  min-width: 120px;
}

.stat-value {
  font-size: 28px;
  font-weight: 700;
  color: var(--text-primary);
}

.stat-label {
  font-size: 12px;
  color: var(--text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.filter-row {
  margin-bottom: 16px;
}

.filter-select {
  padding: 8px 12px;
  border: 1px solid var(--border);
  border-radius: 8px;
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 13px;
}

.data-table {
  width: 100%;
  border-collapse: collapse;
}

.data-table th,
.data-table td {
  padding: 10px 14px;
  text-align: left;
  border-bottom: 1px solid var(--border);
  font-size: 13px;
}

.data-table th {
  color: var(--text-secondary);
  font-weight: 600;
  font-size: 12px;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.clickable {
  cursor: pointer;
}

.clickable:hover {
  background: var(--bg-hover);
}

.loading,
.empty {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary);
}

.back-btn {
  background: none;
  color: var(--accent);
  font-size: 14px;
  margin-bottom: 16px;
  cursor: pointer;
}

.back-btn:hover {
  text-decoration: underline;
}

.detail-content h2 {
  font-size: 20px;
  margin-bottom: 16px;
}

.detail-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 8px 24px;
  margin-bottom: 24px;
}

.detail-content h3 {
  font-size: 16px;
  margin-bottom: 8px;
  margin-top: 16px;
}

.admin-list {
  list-style: disc;
  padding-left: 20px;
}

.admin-list li {
  padding: 4px 0;
}

.ban-row {
  display: flex;
  gap: 16px;
  padding: 8px 0;
  border-bottom: 1px solid var(--border);
  align-items: center;
  font-size: 13px;
}
</style>
