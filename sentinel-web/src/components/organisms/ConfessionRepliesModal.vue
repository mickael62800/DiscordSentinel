<script setup lang="ts">
import { useConfessions } from "@/composables/useConfessions";
import { useConfirm } from "@/composables/useConfirm";
import type { ConfessionReply } from "@/services/confessionsService";
import { useFormatDate } from "@/composables/useFormatDate";

const { repliesTarget, replies, closeReplies, deleteReply } = useConfessions();
const { confirm } = useConfirm();
const { formatDateTimeShort } = useFormatDate();

function fmtDate(iso: string | null): string {
  if (!iso) return "—";
  return formatDateTimeShort(iso);
}

async function onDelete(r: ConfessionReply) {
  const ok = await confirm({ title: "Supprimer reply", message: "Confirmer ?" });
  if (!ok) return;
  await deleteReply(r);
}
</script>

<template>
  <div v-if="repliesTarget" class="modal-overlay" @click.self="closeReplies">
    <div class="modal-card">
      <header class="modal-head">
        <h3>Replies de la confession #{{ repliesTarget.public_number }}</h3>
        <button class="modal-close" @click="closeReplies">×</button>
      </header>
      <div class="modal-body">
        <p v-if="replies.length === 0" class="muted">Aucune reply.</p>
        <div v-for="r in replies" :key="r.id" class="reply-row" :class="{ deleted: r.deleted_at }">
          <div class="reply-head">
            <strong>#{{ r.public_number }}</strong>
            <span v-if="r.is_anonymous" class="badge">anonyme</span>
            <span v-else class="badge">{{ r.author_user_id }}</span>
            <span class="muted small">{{ fmtDate(r.created_at) }}</span>
            <button v-if="!r.deleted_at" class="btn-danger xs" @click="onDelete(r)">🗑</button>
          </div>
          <div class="reply-content">
            <span v-if="r.deleted_at" class="muted">[supprimée]</span>
            <span v-else>{{ r.content }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.muted { color: var(--text-secondary); }
.small { font-size: 12px; }
.badge { display: inline-block; padding: 2px 6px; border-radius: 4px; background: var(--bg-secondary); color: var(--text-secondary); font-size: 10px; margin-left: 6px; text-transform: uppercase; letter-spacing: .5px; }
.modal-overlay { position: fixed; inset: 0; z-index: 1000; background: rgba(0,0,0,.6); display: flex; align-items: center; justify-content: center; padding: 20px; }
.modal-card { background: var(--bg-card); border: 1px solid var(--border); border-radius: 12px; width: 100%; max-width: 720px; max-height: 90vh; display: flex; flex-direction: column; }
.modal-head { display: flex; justify-content: space-between; align-items: center; padding: 16px 20px; border-bottom: 1px solid var(--border); }
.modal-head h3 { margin: 0; font-size: 16px; }
.modal-close { background: transparent; border: 0; cursor: pointer; font-size: 24px; line-height: 1; color: var(--text-secondary); padding: 0 6px; }
.modal-body { padding: 16px 20px; overflow-y: auto; flex: 1; }
.reply-row { padding: 8px 0; border-bottom: 1px solid color-mix(in srgb, var(--border) 60%, transparent); }
.reply-row.deleted { opacity: .5; }
.reply-head { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
.reply-head .btn-danger { margin-left: auto; }
.reply-content { font-size: 13px; padding: 4px 0 4px 16px; word-wrap: break-word; }
.btn-danger { padding: 4px 8px; border-radius: 6px; cursor: pointer; font-size: 11px; font-weight: 600; border: 1px solid var(--border); background: transparent; color: var(--danger, #ef4444); border-color: color-mix(in srgb, var(--danger, #ef4444) 50%, var(--border)); }
.btn-danger:hover { background: color-mix(in srgb, var(--danger, #ef4444) 12%, transparent); }
.xs { padding: 4px 8px; }
</style>
