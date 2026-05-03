<script setup lang="ts">
import { useWheelAnalytics } from "@/composables/useWheelAnalytics";

const { distribution, totalSpins, loading } = useWheelAnalytics();
</script>

<template>
  <section class="card">
    <h2>🎲 Distribution des cases</h2>
    <div v-if="loading" class="loading">Chargement…</div>
    <div v-else-if="distribution.length === 0" class="empty">Aucun spin récent.</div>
    <table v-else class="table">
      <thead>
        <tr>
          <th>Case</th>
          <th>Tombée</th>
          <th>%</th>
          <th>Payout total</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="d in distribution" :key="d.case_key">
          <td>
            <strong>{{ d.label }}</strong>
            <small class="muted">{{ d.case_key }}</small>
          </td>
          <td>{{ d.count }}</td>
          <td>{{ ((d.count / totalSpins) * 100).toFixed(1) }}%</td>
          <td>{{ d.total_payout.toLocaleString() }}c</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
@import "../pages/_moderation-advanced-shared.css";
</style>
