<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { useTicketDetail } from "../../composables/useTickets";
import { useFormatDate } from "../../composables/useFormatDate";
import { statusVariant, priorityVariant } from "../../utils/variants";
import AppBadge from "../atoms/AppBadge.vue";
import AppButton from "../atoms/AppButton.vue";

const props = defineProps<{
  ticketId: string;
}>();

const emit = defineEmits<{ back: [] }>();

const { formatShortDateTime: fmt } = useFormatDate();
const { detail, loading: detailLoading, replying, fetchDetail, reply, close } = useTicketDetail();

const replyContent = ref("");

watch(() => props.ticketId, async (id) => {
  replyContent.value = "";
  await fetchDetail(id);
});

onMounted(async () => {
  await fetchDetail(props.ticketId);
});

async function sendReply() {
  if (!replyContent.value.trim()) return;
  await reply(props.ticketId, replyContent.value);
  replyContent.value = "";
}

async function closeTicket() {
  await close(props.ticketId);
}
</script>

<template>
  <div>
    <div v-if="detailLoading" class="loading">Chargement du ticket...</div>

    <template v-else-if="detail">
      <div class="detail-header">
        <button class="back-btn" @click="emit('back')">&larr; Retour</button>
        <div class="detail-title-row">
          <h1>{{ detail.ticket.title }}</h1>
          <div class="detail-badges">
            <AppBadge :label="detail.ticket.status" :variant="statusVariant(detail.ticket.status)" />
            <AppBadge :label="detail.ticket.priority" :variant="priorityVariant(detail.ticket.priority)" />
            <AppBadge :label="detail.ticket.category" variant="default" />
          </div>
        </div>
        <div class="detail-info">
          <span>Par <strong>{{ detail.ticket.author_name }}</strong></span>
          <span class="sep">dans</span>
          <span><strong>{{ detail.ticket.server }}</strong></span>
          <span class="sep">|</span>
          <span>Assigne a : <strong>{{ detail.ticket.assigned_to ?? "Non assigne" }}</strong></span>
          <span class="sep">|</span>
          <span>Cree le {{ fmt(detail.ticket.created_at) }}</span>
        </div>
      </div>

      <div class="messages-container">
        <div
          v-for="msg in detail.messages"
          :key="msg.id"
          :class="['message', msg.author_role === 'moderator' ? 'message--staff' : 'message--user']"
        >
          <div class="message-header">
            <span class="message-author">{{ msg.author_name }}</span>
            <AppBadge
              v-if="msg.author_role !== 'user'"
              :label="msg.author_role"
              variant="info"
            />
            <span class="message-time">{{ fmt(msg.created_at) }}</span>
          </div>
          <p class="message-content">{{ msg.content }}</p>
        </div>
      </div>

      <div v-if="detail.ticket.status !== 'closed'" class="card reply-box">
        <textarea
          v-model="replyContent"
          placeholder="Tapez votre reponse..."
          rows="3"
          @keydown.ctrl.enter="sendReply"
        ></textarea>
        <div class="reply-actions">
          <AppButton variant="primary" :disabled="!replyContent.trim() || replying" @click="sendReply">
            {{ replying ? "Envoi..." : "Repondre" }}
          </AppButton>
          <AppButton variant="secondary" @click="closeTicket">
            Fermer le ticket
          </AppButton>
        </div>
      </div>

      <div v-else class="closed-notice">
        Ce ticket est ferme.
      </div>
    </template>
  </div>
</template>

<style scoped>
.loading {
  text-align: center;
  color: var(--text-secondary);
  padding: 32px;
}

.detail-header { margin-bottom: 24px; }

.back-btn {
  background: none;
  border: none;
  color: var(--text-secondary);
  padding: 4px 0;
  margin-bottom: 12px;
  font-size: 13px;
  cursor: pointer;
}
.back-btn:hover { color: var(--text-primary); }

.detail-title-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
  flex-wrap: wrap;
}
.detail-title-row h1 { margin: 0; font-size: 22px; }
.detail-badges { display: flex; gap: 6px; }

.detail-info {
  font-size: 13px;
  color: var(--text-secondary);
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}
.detail-info .sep { opacity: 0.4; }

.messages-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin: 0 auto 20px;
  max-width: 900px;
  width: 100%;
  padding: var(--space-xl);
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  box-shadow: var(--shadow-sm);
}

.message {
  padding: 16px;
  border-radius: 12px;
  max-width: 75%;
}
.message--user {
  background-color: var(--bg-primary);
  border: 1px solid var(--border);
  align-self: flex-start;
}
.message--staff {
  background-color: var(--accent-bg);
  border: 1px solid rgba(88, 101, 242, 0.25);
  align-self: flex-end;
}

.message-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}
.message-author { font-weight: 600; font-size: 13px; }
.message-time {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  margin-left: auto;
}
.message-content {
  font-size: 14px;
  line-height: 1.6;
  color: var(--text-primary);
}

.reply-box {
  max-width: 900px;
  width: 100%;
  margin: 0 auto;
}
.reply-box textarea {
  width: 100%;
  background-color: var(--bg-primary);
  border: 1px solid var(--border);
  border-radius: 8px;
  padding: 12px;
  color: var(--text-primary);
  font-family: inherit;
  font-size: 14px;
  resize: vertical;
  outline: none;
  transition: border-color var(--transition-base);
}
.reply-box textarea:focus { border-color: var(--accent); }

.reply-actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
  justify-content: flex-end;
}

.closed-notice {
  text-align: center;
  padding: 20px;
  color: var(--text-secondary);
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: var(--radius-lg);
  max-width: 900px;
  width: 100%;
  margin: 0 auto;
}
</style>
