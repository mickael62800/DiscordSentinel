<script setup lang="ts">
import { useWheelAnalytics } from "@/composables/useWheelAnalytics";

const { spins, loading } = useWheelAnalytics();

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR", {
    day: "2-digit", month: "2-digit",
    hour: "2-digit", minute: "2-digit",
  });
}
</script>

<template>
  <section class="card">
    <h2>⏱️ Spins récents (50)</h2>
    <div v-if="loading" class="loading">Chargement…</div>
    <div v-else-if="spins.length === 0" class="empty">Aucun spin récent.</div>
    <table v-else class="table">
      <thead>
        <tr>
          <th>Heure</th>
          <th>Joueur</th>
          <th>Case</th>
          <th>Payout</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="s in spins" :key="s.id">
          <td>{{ formatDate(s.created_at) }}</td>
          <td>{{ s.username }}</td>
          <td>
            <strong>{{ s.case_label }}</strong>
            <small class="muted">{{ s.case_key }}</small>
          </td>
          <td>
            <span :class="{ pos: s.payout > 0, neg: s.payout < 0 }">
              {{ s.payout > 0 ? '+' : '' }}{{ s.payout.toLocaleString() }}c
            </span>
          </td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
@import "../pages/_moderation-advanced-shared.css";
.pos { color: #2ECC71; font-weight: 600; }
.neg { color: #E74C3C; font-weight: 600; }
</style>
