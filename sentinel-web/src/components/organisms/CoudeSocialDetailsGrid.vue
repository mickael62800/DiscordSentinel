<script setup lang="ts">
import { useCoudeSocial } from "@/composables/useCoudeSocial";

const { curse, bounty, coalition, vendettasAsChallenger, liftCurse } = useCoudeSocial();

function formatDate(iso: string | null): string {
  if (!iso) return "—";
  return new Date(iso).toLocaleString("fr-FR");
}

function statusColor(s: string): string {
  if (s === "active" || s === "open") return "#F1C40F";
  if (s === "resolved" || s === "won" || s === "claimed") return "#2ECC71";
  if (s === "broken" || s === "lost" || s === "lifted") return "#E74C3C";
  if (s === "expired") return "#7F8C8D";
  return "#888";
}
</script>

<template>
  <div class="grid">
    <!-- Curse -->
    <section class="card">
      <h2>🌶️ Malédiction active</h2>
      <div v-if="!curse" class="empty">Aucune malédiction active.</div>
      <div v-else class="curse-card">
        <div class="curse-header">
          <span class="curse-emoji">{{ curse.kind_emoji }}</span>
          <strong>{{ curse.kind_label }}</strong>
          <code>{{ curse.kind }}</code>
        </div>
        <p>
          <em>Lancée par</em> <code>{{ curse.source_id }}</code> le {{ formatDate(curse.created_at) }}.
        </p>
        <p>Expire : {{ formatDate(curse.expires_at) }}</p>
        <button class="btn-warn" @click="liftCurse">🛡️ Lever (admin override)</button>
      </div>
    </section>

    <!-- Bounty -->
    <section class="card">
      <h2>💰 Prime collective</h2>
      <div v-if="!bounty" class="empty">Aucune prime ouverte sur ce joueur.</div>
      <div v-else>
        <div class="kv"><span>Pot total</span><strong>{{ bounty.total_amount.toLocaleString() }} coins</strong></div>
        <div class="kv">
          <span>Statut</span>
          <span class="badge" :style="{ backgroundColor: statusColor(bounty.status) }">{{ bounty.status }}</span>
        </div>
        <div class="kv"><span>Ouverte le</span><span>{{ formatDate(bounty.opened_at) }}</span></div>
        <div v-if="bounty.claimed_by" class="kv"><span>Claimée par</span><code>{{ bounty.claimed_by }}</code></div>
        <div v-if="bounty.claimed_at" class="kv"><span>Le</span><span>{{ formatDate(bounty.claimed_at) }}</span></div>
      </div>
    </section>

    <!-- Coalition -->
    <section class="card">
      <h2>🤝 Coalition contre</h2>
      <div v-if="!coalition" class="empty">Aucune coalition active.</div>
      <div v-else>
        <div class="kv">
          <span>Statut</span>
          <span class="badge" :style="{ backgroundColor: statusColor(coalition.status) }">{{ coalition.status }}</span>
        </div>
        <div class="kv"><span>Ouverte</span><span>{{ formatDate(coalition.opened_at) }}</span></div>
        <div class="kv"><span>Expire</span><span>{{ formatDate(coalition.expires_at) }}</span></div>
        <div v-if="coalition.broken_by" class="kv"><span>Cassée par</span><code>{{ coalition.broken_by }}</code></div>
        <h4>Membres ({{ coalition.members.length }})</h4>
        <ul class="members-list">
          <li v-for="m in coalition.members" :key="m.member_id">
            <strong>{{ m.member_name }}</strong>
            <code>{{ m.member_id }}</code>
            <span class="muted">{{ formatDate(m.joined_at) }}</span>
          </li>
        </ul>
      </div>
    </section>

    <!-- Vendettas -->
    <section class="card">
      <h2>⚔️ Vendettas (en tant que challenger)</h2>
      <div v-if="vendettasAsChallenger.length === 0" class="empty">
        Aucune vendetta en tant que challenger.
      </div>
      <ul v-else class="vendetta-list">
        <li v-for="v in vendettasAsChallenger" :key="v.id">
          <div class="kv"><span>Cible</span><code>{{ v.target_id }}</code></div>
          <div class="kv">
            <span>Statut</span>
            <span class="badge" :style="{ backgroundColor: statusColor(v.status) }">{{ v.status }}</span>
          </div>
          <div class="kv"><span>Déclarée</span><span>{{ formatDate(v.declared_at) }}</span></div>
          <div class="kv"><span>Expire</span><span>{{ formatDate(v.expires_at) }}</span></div>
          <div v-if="v.resolved_at" class="kv"><span>Résolue</span><span>{{ formatDate(v.resolved_at) }}</span></div>
        </li>
      </ul>
    </section>
  </div>
</template>

<style scoped>
@import "../pages/_admin-page-shared.css";
.grid { display: grid; grid-template-columns: 1fr 1fr; gap: 16px; }
.kv {
  display: flex;
  justify-content: space-between;
  padding: 4px 0;
  border-bottom: 1px solid var(--border);
  font-size: 0.9rem;
}
.kv:last-child { border-bottom: none; }
.kv span:first-child { color: var(--text-secondary); }
.curse-card { display: flex; flex-direction: column; gap: 8px; }
.curse-header { display: flex; align-items: center; gap: 8px; }
.curse-emoji { font-size: 1.4rem; }
.curse-header code {
  margin-left: auto;
  font-size: 0.8rem;
  color: var(--text-secondary);
}
.members-list, .vendetta-list { list-style: none; padding: 0; margin: 8px 0 0 0; }
.members-list li {
  display: grid;
  grid-template-columns: 1fr 1fr auto;
  gap: 8px;
  padding: 6px 0;
  border-bottom: 1px solid var(--border);
  font-size: 0.9rem;
  align-items: center;
}
.vendetta-list li {
  background: var(--bg-card);
  padding: 8px 12px;
  border-radius: 4px;
  margin-bottom: 8px;
}
h4 { margin: 12px 0 4px 0; font-size: 0.95rem; color: var(--text-secondary); }
</style>
