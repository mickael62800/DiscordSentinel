<script setup lang="ts">
import { ref } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { httpGet } from "@/api/http";

interface NameHistoryEntry {
  id: string;
  guild_id: string;
  user_id: string;
  old_name: string;
  new_name: string;
  created_at: string;
}

const { guildIdFilter } = useGuildSelector();
const { error: showError } = useToast();
const lookupUserId = ref("");
const entries = ref<NameHistoryEntry[]>([]);
const loading = ref(false);

async function lookup() {
  if (!guildIdFilter.value || !lookupUserId.value.trim()) {
    entries.value = [];
    return;
  }
  loading.value = true;
  try {
    entries.value = await httpGet<NameHistoryEntry[]>(
      `/api/name-history/${guildIdFilter.value}/${lookupUserId.value.trim()}`,
    );
  } catch (e) {
    console.error(e);
    showError("Erreur chargement historique pseudos.");
    entries.value = [];
  } finally {
    loading.value = false;
  }
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR");
}
</script>

<template>
  <div class="page">
    <header class="page-header">
      <h1>📛 Historique des pseudos</h1>
      <p class="lede">
        Liste des changements de pseudo Discord d'un membre — utile pour
        détecter les évasions, les usurpations d'identité ou simplement
        suivre l'historique d'un compte. Les changements sont collectés
        automatiquement par <code>audit-bot</code>.
      </p>
    </header>

    <section class="card">
      <h2>Recherche</h2>
      <div class="lookup">
        <input
          v-model="lookupUserId"
          placeholder="ID Discord de l'utilisateur"
          @keyup.enter="lookup"
        />
        <button class="btn-secondary" @click="lookup">Rechercher</button>
      </div>
    </section>

    <section v-if="lookupUserId" class="card">
      <h2>Historique ({{ entries.length }})</h2>
      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="entries.length === 0" class="empty">
        Aucun changement de pseudo enregistré pour cet utilisateur.
      </div>
      <ol v-else class="timeline">
        <li v-for="(e, idx) in entries" :key="e.id">
          <div class="time">{{ formatDate(e.created_at) }}</div>
          <div class="change">
            <span class="old">{{ e.old_name || "(vide)" }}</span>
            <span class="arrow">→</span>
            <span class="new">{{ e.new_name || "(vide)" }}</span>
          </div>
          <div v-if="idx === 0" class="badge current">actuel</div>
        </li>
      </ol>
    </section>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
.timeline {
  list-style: none;
  padding: 0;
  margin: 0;
}
.timeline li {
  display: grid;
  grid-template-columns: 180px 1fr auto;
  gap: 16px;
  align-items: center;
  padding: 12px 0;
  border-bottom: 1px solid var(--border);
}
.timeline li:last-child {
  border-bottom: none;
}
.time {
  font-size: 0.85rem;
  color: var(--text-secondary);
  font-family: monospace;
}
.change {
  display: flex;
  align-items: center;
  gap: 12px;
}
.old {
  color: var(--text-secondary);
  text-decoration: line-through;
  font-size: 0.95rem;
}
.arrow {
  color: var(--text-secondary);
  font-size: 1.1rem;
}
.new {
  font-weight: 600;
  font-size: 0.95rem;
}
.badge.current {
  background: #2ECC71;
  color: white;
  padding: 2px 8px;
  border-radius: 12px;
  font-size: 0.75rem;
  font-weight: 600;
}
</style>
