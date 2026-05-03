<script setup lang="ts">
import { useWheelAnalytics } from "@/composables/useWheelAnalytics";

const { topWinners, loading } = useWheelAnalytics();
</script>

<template>
  <section class="card">
    <h2>🏆 Top 10 (7 jours)</h2>
    <div v-if="loading" class="loading">Chargement…</div>
    <div v-else-if="topWinners.length === 0" class="empty">Aucun gagnant sur 7 jours.</div>
    <table v-else class="table">
      <thead>
        <tr>
          <th>#</th>
          <th>Joueur</th>
          <th>Gain total</th>
          <th>Spins</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="(w, idx) in topWinners" :key="w.user_id">
          <td>{{ idx + 1 }}</td>
          <td>
            <strong>{{ w.username }}</strong>
            <small class="muted">{{ w.user_id }}</small>
          </td>
          <td><strong>{{ w.total_payout.toLocaleString() }}c</strong></td>
          <td>{{ w.spin_count }}</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
@import "../pages/_moderation-advanced-shared.css";
</style>
