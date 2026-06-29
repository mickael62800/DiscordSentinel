<script setup lang="ts">
import { computed } from "vue";
import { useConfessions } from "@/composables/useConfessions";
import { useConfirm } from "@/composables/useConfirm";
import { useMyRole } from "@/composables/useMyRole";
import type { Confession } from "@/services/confessionsService";
import { useFormatDate } from "@/composables/useFormatDate";

const { confessions, showReplies, deleteConfession } = useConfessions();
const { confirm } = useConfirm();
const { isSuper, role } = useMyRole();
const isOwner = computed(() => isSuper.value || role.value === "owner");
const { formatDateTimeShort } = useFormatDate();

function fmtDate(iso: string | null): string {
  if (!iso) return "—";
  return formatDateTimeShort(iso);
}

function truncate(s: string, n = 100): string {
  return s.length > n ? s.slice(0, n) + "…" : s;
}

async function onDelete(c: Confession) {
  const ok = await confirm({
    title: `Supprimer #${c.public_number}`,
    message: `Supprimer définitivement la confession #${c.public_number} ? Le message Discord sera aussi retiré.`,
  });
  if (!ok) return;
  await deleteConfession(c);
}
</script>

<template>
  <table v-if="confessions.length > 0" class="data-table">
    <thead>
      <tr>
        <th>#</th>
        <th>Date</th>
        <th>Auteur</th>
        <th>Contenu</th>
        <th>État</th>
        <th class="actions-h">Actions</th>
      </tr>
    </thead>
    <tbody>
      <tr v-for="c in confessions" :key="c.id" :class="{ deleted: c.deleted_at }">
        <td class="mono">#{{ c.public_number }}</td>
        <td class="small mono">{{ fmtDate(c.created_at) }}</td>
        <td class="small mono">
          <span v-if="isOwner">{{ c.author_user_id }}</span>
          <span v-else class="muted">[anonyme]</span>
        </td>
        <td class="small">
          {{ truncate(c.content, 100) }}
          <span v-if="c.edited_at" class="badge">éd.</span>
        </td>
        <td>
          <span v-if="c.deleted_at" class="badge danger">supprimée</span>
          <span v-else class="badge">active</span>
        </td>
        <td class="actions">
          <button class="btn-secondary xs" @click="showReplies(c)" title="Voir les replies">💬</button>
          <button v-if="!c.deleted_at" class="btn-danger xs" @click="onDelete(c)" title="Supprimer">🗑</button>
        </td>
      </tr>
    </tbody>
  </table>
  <p v-else class="empty">Aucune confession.</p>
</template>

<style scoped>
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
.mono { font-family: "JetBrains Mono", monospace; }
.empty { padding: 30px; text-align: center; color: var(--text-secondary); font-style: italic; }
.data-table { width: 100%; border-collapse: collapse; }
.data-table th, .data-table td { padding: 10px 12px; border-bottom: 1px solid var(--border); }
.data-table th { text-align: left; font-size: 11px; text-transform: uppercase; color: var(--text-secondary); letter-spacing: .5px; }
.data-table tr.deleted { opacity: .5; }
.data-table .actions-h, .data-table .actions { text-align: right; white-space: nowrap; }
.data-table .actions button { margin-left: 4px; }
.badge { display: inline-block; padding: 2px 6px; border-radius: 4px; background: var(--bg-secondary); color: var(--text-secondary); font-size: 10px; margin-left: 6px; text-transform: uppercase; letter-spacing: .5px; }
.badge.danger { background: rgba(231, 76, 60, .15); color: #e74c3c; }
.btn-secondary, .btn-danger { padding: 4px 8px; border-radius: 6px; cursor: pointer; font-size: 11px; font-weight: 600; border: 1px solid var(--border); background: transparent; color: var(--text-primary); }
.btn-secondary:hover { background: var(--bg-hover); }
.btn-danger { color: var(--danger, #ef4444); border-color: color-mix(in srgb, var(--danger, #ef4444) 50%, var(--border)); }
.btn-danger:hover { background: color-mix(in srgb, var(--danger, #ef4444) 12%, transparent); }
.xs { padding: 4px 8px; }
</style>
