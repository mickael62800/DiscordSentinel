<script setup lang="ts">
import { useAutomod } from "@/composables/useAutomod";

const {
  detections,
  statsByCategory,
  topUsers,
  totalDetections,
  loading,
  userFilter,
  fetchDetections,
} = useAutomod();

function formatDate(iso: string): string {
  const d = new Date(iso);
  return d.toLocaleString("fr-FR", {
    day: "2-digit",
    month: "2-digit",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function severityLabel(s: number): { label: string; color: string } {
  if (s >= 8) return { label: "Critique", color: "#E74C3C" };
  if (s >= 5) return { label: "Élevée", color: "#E67E22" };
  if (s >= 2) return { label: "Moyenne", color: "#F1C40F" };
  return { label: "Faible", color: "#7F8C8D" };
}
</script>

<template>
  <div class="automod-page">
    <header class="page-header">
      <h1>🤖 Automod</h1>
      <p class="lede">
        Timeline des détections automatiques (spam, insultes, liens, phishing,
        mentions massives, fichiers suspects, unicode invisible…). La
        configuration des seuils et des détecteurs se fait dans
        <router-link to="/component-config">Component config</router-link> →
        <em>automod-bot</em>.
      </p>
    </header>

    <section class="kpi-row">
      <div class="kpi-card">
        <span class="kpi-value">{{ totalDetections }}</span>
        <span class="kpi-label">Détections récentes</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ statsByCategory.length }}</span>
        <span class="kpi-label">Catégories distinctes</span>
      </div>
      <div class="kpi-card">
        <span class="kpi-value">{{ topUsers.length }}</span>
        <span class="kpi-label">Utilisateurs détectés (top 10)</span>
      </div>
    </section>

    <div class="grid">
      <!-- Stats par catégorie -->
      <section class="card">
        <h2>Catégories</h2>
        <div v-if="statsByCategory.length === 0" class="empty">
          Aucune détection.
        </div>
        <ul v-else class="cat-list">
          <li v-for="cat in statsByCategory" :key="cat.key">
            <span class="cat-name">{{ cat.key }}</span>
            <span class="cat-count">{{ cat.count }}</span>
          </li>
        </ul>
      </section>

      <!-- Top users -->
      <section class="card">
        <h2>Top utilisateurs</h2>
        <div v-if="topUsers.length === 0" class="empty">
          Aucune détection.
        </div>
        <ul v-else class="user-list">
          <li v-for="user in topUsers" :key="user.user_id">
            <span class="user-name">{{ user.username }}</span>
            <span class="user-id">{{ user.user_id }}</span>
            <span class="user-count">{{ user.count }}</span>
          </li>
        </ul>
      </section>
    </div>

    <section class="card timeline">
      <div class="timeline-header">
        <h2>Timeline des détections</h2>
        <div class="filters">
          <input
            v-model="userFilter"
            placeholder="Filtrer par user ID"
            @keyup.enter="fetchDetections"
          />
          <button class="btn-secondary" @click="fetchDetections">Filtrer</button>
        </div>
      </div>

      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="detections.length === 0" class="empty">
        Aucune détection à afficher.
      </div>
      <table v-else class="detections-table">
        <thead>
          <tr>
            <th>Date</th>
            <th>Utilisateur</th>
            <th>Raison</th>
            <th>Sévérité</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="d in detections" :key="d.id">
            <td>{{ formatDate(d.created_at) }}</td>
            <td>
              <strong>{{ d.username }}</strong>
              <small class="muted">{{ d.user_id }}</small>
            </td>
            <td class="reason">{{ d.reason }}</td>
            <td>
              <span
                class="severity-badge"
                :style="{ backgroundColor: severityLabel(d.score ?? 0).color }"
              >
                {{ severityLabel(d.score ?? 0).label }}
              </span>
              <small class="muted">{{ (d.score ?? 0).toFixed(1) }}</small>
            </td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>

<style scoped>
.automod-page {
  max-width: 1200px;
  margin: 0 auto;
  padding: 24px;
}
.page-header {
  margin-bottom: 24px;
}
.page-header h1 {
  margin: 0 0 8px 0;
  font-size: 1.6rem;
}
.lede {
  color: var(--text-muted, #888);
  margin: 0;
}
.lede a {
  color: #5865F2;
  text-decoration: none;
}
.lede a:hover {
  text-decoration: underline;
}
.kpi-row {
  display: grid;
  grid-template-columns: repeat(3, 1fr);
  gap: 12px;
  margin-bottom: 20px;
}
.kpi-card {
  background: var(--bg-card, #1f1f1f);
  border: 1px solid var(--border-color, #333);
  border-radius: 8px;
  padding: 16px 20px;
  display: flex;
  flex-direction: column;
}
.kpi-value {
  font-size: 1.8rem;
  font-weight: 700;
}
.kpi-label {
  font-size: 0.85rem;
  color: var(--text-muted, #888);
  margin-top: 4px;
}
.grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 20px;
  margin-bottom: 20px;
}
.card {
  background: var(--bg-card, #1f1f1f);
  border: 1px solid var(--border-color, #333);
  border-radius: 8px;
  padding: 20px;
}
.card h2 {
  margin: 0 0 12px 0;
  font-size: 1.1rem;
}
.empty {
  color: var(--text-muted, #888);
  font-style: italic;
}
.cat-list,
.user-list {
  list-style: none;
  padding: 0;
  margin: 0;
}
.cat-list li,
.user-list li {
  display: flex;
  justify-content: space-between;
  padding: 6px 0;
  border-bottom: 1px solid var(--border-color, #333);
}
.cat-list li:last-child,
.user-list li:last-child {
  border-bottom: none;
}
.cat-count,
.user-count {
  font-weight: 600;
  color: #5865F2;
}
.user-list li {
  display: grid;
  grid-template-columns: 2fr 2fr 1fr;
  gap: 8px;
  align-items: center;
}
.user-id {
  font-family: monospace;
  font-size: 0.85rem;
  color: var(--text-muted, #888);
}
.user-count {
  text-align: right;
}
.timeline-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}
.filters {
  display: flex;
  gap: 8px;
}
.filters input {
  background: var(--bg-input, #2a2a2a);
  border: 1px solid var(--border-color, #444);
  border-radius: 4px;
  padding: 6px 10px;
  color: inherit;
}
.btn-secondary {
  background: #5865F2;
  color: white;
  border: none;
  border-radius: 4px;
  padding: 6px 14px;
  cursor: pointer;
}
.detections-table {
  width: 100%;
  border-collapse: collapse;
}
.detections-table th,
.detections-table td {
  text-align: left;
  padding: 8px 10px;
  border-bottom: 1px solid var(--border-color, #333);
  vertical-align: top;
}
.detections-table th {
  font-size: 0.85rem;
  font-weight: 600;
  color: var(--text-muted, #888);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}
.detections-table small.muted {
  display: block;
  font-size: 0.75rem;
  color: var(--text-muted, #888);
  margin-top: 2px;
}
.severity-badge {
  display: inline-block;
  padding: 2px 8px;
  border-radius: 12px;
  color: white;
  font-size: 0.75rem;
  font-weight: 600;
}
.reason {
  max-width: 480px;
  word-break: break-word;
}
.loading {
  padding: 32px;
  text-align: center;
  color: var(--text-muted, #888);
}
</style>
