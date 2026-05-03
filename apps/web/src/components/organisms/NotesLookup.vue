<script setup lang="ts">
import { useNotes } from "@/composables/useNotes";

const { lookupUserId, fetch } = useNotes();

async function onSearch() {
  await fetch(lookupUserId.value);
}
</script>

<template>
  <section class="card">
    <h2>Recherche</h2>
    <div class="lookup">
      <input
        v-model="lookupUserId"
        placeholder="ID de l'utilisateur"
        @keyup.enter="onSearch"
      />
      <button class="btn-secondary" @click="onSearch">Rechercher</button>
    </div>
  </section>
</template>

<style scoped>
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg, 12px);
  padding: 20px;
  margin-bottom: 20px;
}
.card h2 { margin: 0 0 16px 0; font-size: 16px; font-weight: 700; }
.lookup { display: flex; gap: 8px; flex-wrap: wrap; }
.lookup input {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md, 8px);
  padding: 8px 12px;
  color: var(--text-primary);
  font-family: inherit; font-size: 13px; font-weight: 500;
  outline: none; flex: 1; min-width: 220px; max-width: 320px;
  transition: border-color .15s, box-shadow .15s;
}
.lookup input:hover { border-color: color-mix(in srgb, var(--accent) 50%, var(--border)); }
.lookup input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent);
}
.btn-secondary {
  border: 1px solid var(--border);
  border-radius: var(--radius-md, 8px);
  padding: 8px 18px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 600;
  background: var(--bg-card);
  color: var(--text-primary);
  transition: all .15s;
}
.btn-secondary:hover { background: var(--bg-hover); border-color: color-mix(in srgb, var(--accent) 50%, var(--border)); }
</style>
