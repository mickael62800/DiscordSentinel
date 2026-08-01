<script setup lang="ts">
import { useStrikes } from "@/composables/useStrikes";
import { useFormatDate } from "@/composables/useFormatDate";

const {
  userStrikes,
  lookupUserId,
  loadingStrikes,
  lookupStrikes,
  resetStrikes,
} = useStrikes();

const { formatDateTimeNumeric: formatDate } = useFormatDate();
</script>

<template>
  <section class="card">
    <h2>Strikes par utilisateur</h2>
    <div class="lookup">
      <input
        v-model="lookupUserId"
        placeholder="ID de l'utilisateur"
        @keyup.enter="lookupStrikes"
      />
      <button class="btn-secondary" @click="lookupStrikes">Rechercher</button>
      <button
        v-if="userStrikes.length > 0"
        class="btn-danger"
        @click="resetStrikes"
      >
        Reset tous les strikes
      </button>
    </div>

    <div v-if="loadingStrikes" class="loading">Chargement…</div>
    <div v-else-if="userStrikes.length === 0 && lookupUserId" class="empty">
      Aucun strike actif pour cet utilisateur.
    </div>
    <table v-else-if="userStrikes.length > 0" class="strikes-table">
      <thead>
        <tr>
          <th>Date</th>
          <th>Raison</th>
          <th>Source</th>
          <th>Expire</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="s in userStrikes" :key="s.id">
          <td>{{ formatDate(s.created_at) }}</td>
          <td class="reason">{{ s.reason }}</td>
          <td><code>{{ s.source }}</code></td>
          <td>{{ s.expires_at ? formatDate(s.expires_at) : "—" }}</td>
        </tr>
      </tbody>
    </table>
  </section>
</template>

<style scoped>
.card {
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-radius: var(--radius-md);
  padding: 20px;
  margin-bottom: 20px;
}
.card h2 { margin: 0 0 12px 0; }
.lookup {
  display: flex; gap: 8px; align-items: center;
  margin-bottom: 16px; flex-wrap: wrap;
}
.lookup input {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-md, 8px);
  padding: 8px 12px;
  color: var(--text-primary);
  font-family: inherit; font-size: 13px; font-weight: 500;
  flex: 1; min-width: 200px; max-width: 320px; outline: none;
  transition: border-color .15s, box-shadow .15s;
}
.lookup input:hover { border-color: color-mix(in srgb, var(--accent) 50%, var(--border)); }
.lookup input:focus {
  border-color: var(--accent);
  box-shadow: 0 0 0 3px color-mix(in srgb, var(--accent) 25%, transparent);
}
.btn-secondary, .btn-danger {
  border: 1px solid transparent; border-radius: var(--radius-md, 8px);
  padding: 8px 18px; cursor: pointer;
  font-size: 13px; font-weight: 600; transition: all .15s;
}
.btn-secondary { background: var(--bg-card); border-color: var(--border); color: var(--text-primary); }
.btn-secondary:hover { background: var(--bg-hover); border-color: color-mix(in srgb, var(--accent) 50%, var(--border)); }
.btn-danger { background: var(--danger); color: white; }
.btn-danger:hover {
  background: color-mix(in srgb, var(--danger) 88%, white);
  box-shadow: 0 4px 14px color-mix(in srgb, var(--danger) 35%, transparent);
}
.loading, .empty { padding: 16px; text-align: center; color: var(--text-secondary); }
.strikes-table { width: 100%; border-collapse: collapse; }
.strikes-table th, .strikes-table td {
  text-align: left; padding: 8px 10px;
  border-bottom: 1px solid var(--border); vertical-align: middle;
}
.strikes-table th {
  font-size: 11px; color: var(--text-secondary);
  text-transform: uppercase; letter-spacing: .6px; font-weight: 700;
}
.reason { max-width: 480px; word-break: break-word; }
</style>
