<script setup lang="ts">
import { useCoudeSocial } from "@/composables/useCoudeSocial";
import { useFormatDate } from "@/composables/useFormatDate";

const { curse, primes, bountyPot, liftCurse } = useCoudeSocial();
const { formatDateTimeShort } = useFormatDate();

function formatDate(iso: string | null): string {
  if (!iso) return "—";
  return formatDateTimeShort(iso);
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

    <!-- Primes / bounties -->
    <section class="card">
      <h2>💰 Primes sur ce joueur</h2>
      <div v-if="primes.length === 0" class="empty">Aucune prime posée sur ce joueur.</div>
      <div v-else>
        <div class="kv">
          <span>Pot total (non réclamé)</span>
          <strong>{{ bountyPot.toLocaleString() }} coins</strong>
        </div>
        <ul class="prime-list">
          <li v-for="p in primes" :key="p.id">
            <div class="kv">
              <span>Posée par</span>
              <strong>{{ p.placed_by_name || p.placed_by_id }}</strong>
            </div>
            <div class="kv"><span>Montant</span><span>{{ p.amount.toLocaleString() }} coins</span></div>
            <div class="kv">
              <span>Statut</span>
              <span class="badge" :style="{ backgroundColor: statusColor(p.claimed ? 'claimed' : 'open') }">
                {{ p.claimed ? "réclamée" : "ouverte" }}
              </span>
            </div>
            <div v-if="p.claimed && p.claimed_by_name" class="kv">
              <span>Réclamée par</span><code>{{ p.claimed_by_name }}</code>
            </div>
            <div class="kv"><span>Posée le</span><span>{{ formatDate(p.created_at) }}</span></div>
          </li>
        </ul>
      </div>
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
.prime-list { list-style: none; padding: 0; margin: 8px 0 0 0; }
.prime-list li {
  background: var(--bg-card);
  padding: 8px 12px;
  border-radius: 4px;
  margin-bottom: 8px;
}
</style>
