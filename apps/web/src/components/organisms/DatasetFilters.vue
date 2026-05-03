<script setup lang="ts">
import { useAiDataset } from "@/composables/useAiDataset";

const {
  filterChannel, filterFrom, filterTo, minLen, limit, offset, loading, fetchData,
} = useAiDataset();

function search() {
  offset.value = 0;
  fetchData();
}
</script>

<template>
  <section class="card filters">
    <div class="filter">
      <label>Channel ID</label>
      <input v-model="filterChannel" placeholder="(facultatif)" />
    </div>
    <div class="filter">
      <label>Du</label>
      <input v-model="filterFrom" type="datetime-local" />
    </div>
    <div class="filter">
      <label>Au</label>
      <input v-model="filterTo" type="datetime-local" />
    </div>
    <div class="filter">
      <label>Longueur min.</label>
      <input v-model.number="minLen" type="number" min="0" />
    </div>
    <div class="filter">
      <label>Par page</label>
      <select v-model.number="limit">
        <option :value="100">100</option>
        <option :value="200">200</option>
        <option :value="500">500</option>
        <option :value="1000">1000</option>
      </select>
    </div>
    <div class="filter actions">
      <button class="btn" :disabled="loading" @click="search">🔍 Rechercher</button>
    </div>
  </section>
</template>

<style scoped>
.card {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
  margin-bottom: 16px;
}
.filters { display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-end; }
.filter { display: flex; flex-direction: column; gap: 4px; }
.filter label { font-size: 11px; color: var(--text-secondary); text-transform: uppercase; }
.filter input, .filter select {
  padding: 6px 10px;
  border-radius: 6px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px;
}
.btn {
  padding: 7px 14px;
  border-radius: 8px;
  border: 1px solid var(--border);
  background: var(--bg-secondary);
  color: var(--text-primary);
  font-size: 12px; font-weight: 600; cursor: pointer;
}
.btn:hover:not(:disabled) { border-color: var(--accent); color: var(--accent); }
.btn:disabled { opacity: 0.5; cursor: not-allowed; }
</style>
