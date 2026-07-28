<script setup lang="ts">
import { computed } from "vue";
import { useRotationDashboard } from "@/composables/useRotationDashboard";
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";

const { state, history, loading } = useRotationDashboard();

const stateLabel = computed(() => {
  switch (state.value?.state) {
    case "offering_candidate": return "Sollicitation d'un modérateur en cours";
    case "awaiting_owner": return "En attente de validation du fondateur";
    case "offering_stay": return "L'admin actuel doit confirmer s'il reste";
    default: return "Au repos";
  }
});

function fmt(d: string | null): string {
  if (!d) return "—";
  return new Date(d).toLocaleString("fr-FR");
}
function rel(d: string | null): string {
  if (!d) return "non planifiée";
  return new Date(d).toLocaleDateString("fr-FR", { day: "numeric", month: "long", year: "numeric" });
}
</script>

<template>
  <AdminPageShell title="Administrateur tournant" icon="👑">
    <template #lede>
      Suivi de la rotation de l'administrateur : qui est admin actuellement,
      l'état de la rotation en cours, la prochaine échéance et l'historique des
      mandats. La configuration (rôles, durée, message) se fait dans la page
      <strong>Composants</strong>. Les actions manuelles se font via la commande
      <code>/rotation</code> sur Discord.
    </template>

    <div v-if="loading" class="loading">Chargement…</div>
    <div v-else-if="!state" class="empty">
      Sélectionne une guild dans le menu en haut.
    </div>
    <div v-else class="content">
      <div class="cards">
        <div class="card">
          <span class="card-label">Administrateur actuel</span>
          <span class="card-value">
            {{ state.current_admin_id ? `Membre ${state.current_admin_id}` : "Aucun" }}
          </span>
          <span class="card-sub">Depuis : {{ fmt(state.current_admin_since) }}</span>
        </div>
        <div class="card">
          <span class="card-label">État</span>
          <span class="card-value">{{ stateLabel }}</span>
          <span v-if="state.candidate_id" class="card-sub">
            Candidat : membre {{ state.candidate_id }}
          </span>
        </div>
        <div class="card">
          <span class="card-label">Prochaine rotation</span>
          <span class="card-value">{{ rel(state.next_rotation_at) }}</span>
        </div>
      </div>

      <h3 class="section-title">Historique des mandats</h3>
      <div v-if="history.length === 0" class="empty-sm">Aucun mandat enregistré pour l'instant.</div>
      <table v-else class="history">
        <thead>
          <tr><th>Membre</th><th>Dernier mandat</th></tr>
        </thead>
        <tbody>
          <tr v-for="h in history" :key="h.user_id">
            <td>Membre {{ h.user_id }}</td>
            <td>{{ fmt(h.served_at) }}</td>
          </tr>
        </tbody>
      </table>
    </div>
  </AdminPageShell>
</template>

<style scoped>
.loading, .empty { padding: 48px; text-align: center; color: var(--text-secondary); font-size: 13px; }
.empty-sm { padding: 16px; color: var(--text-secondary); font-size: 13px; }
.cards { display: flex; flex-wrap: wrap; gap: 12px; margin-bottom: 24px; }
.card {
  flex: 1 1 200px;
  display: flex;
  flex-direction: column;
  gap: 6px;
  padding: 16px;
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 10px;
}
.card-label { font-size: 12px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; }
.card-value { font-size: 18px; font-weight: 700; color: var(--text-primary); }
.card-sub { font-size: 12px; color: var(--text-secondary); }
.section-title { font-size: 14px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.5px; margin: 8px 0 12px; }
.history { width: 100%; border-collapse: collapse; }
.history th, .history td {
  text-align: left;
  padding: 10px 12px;
  border-bottom: 1px solid var(--border);
  font-size: 13px;
  color: var(--text-primary);
}
.history th { color: var(--text-secondary); font-weight: 600; }
</style>
