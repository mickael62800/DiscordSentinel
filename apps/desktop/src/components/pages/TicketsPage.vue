<script setup lang="ts">
import { ref, computed } from "vue";
import { useTickets, useTicketDetail } from "../../composables/useTickets";
import AppBadge from "../atoms/AppBadge.vue";
import AppButton from "../atoms/AppButton.vue";
import FilterBar from "../molecules/FilterBar.vue";

const { filteredTickets, loading, filterStatus, filterPriority, openCount, pendingCount } = useTickets();
const { detail, loading: detailLoading, replying, fetchDetail, reply, close } = useTicketDetail();

const selectedId = ref<string | null>(null);
const replyContent = ref("");

const filters = computed(() => [
  {
    modelValue: filterStatus.value,
    options: [
      { value: "all", label: "All statuses" },
      { value: "open", label: "Open" },
      { value: "pending", label: "Pending" },
      { value: "closed", label: "Closed" },
    ],
  },
  {
    modelValue: filterPriority.value,
    options: [
      { value: "all", label: "All priorities" },
      { value: "urgent", label: "Urgent" },
      { value: "high", label: "High" },
      { value: "medium", label: "Medium" },
      { value: "low", label: "Low" },
    ],
  },
]);

function onFilterUpdate(index: number, value: string) {
  if (index === 0) filterStatus.value = value;
  if (index === 1) filterPriority.value = value;
}

async function selectTicket(id: string) {
  selectedId.value = id;
  replyContent.value = "";
  await fetchDetail(id);
}

async function sendReply() {
  if (!replyContent.value.trim() || !selectedId.value) return;
  await reply(selectedId.value, replyContent.value);
  replyContent.value = "";
}

async function closeTicket() {
  if (!selectedId.value) return;
  await close(selectedId.value);
}

function backToList() {
  selectedId.value = null;
  detail.value = null;
}

function statusVariant(status: string): "success" | "warning" | "info" | "default" {
  switch (status) {
    case "open": return "info";
    case "pending": return "warning";
    case "closed": return "success";
    default: return "default";
  }
}

function priorityVariant(priority: string): "danger" | "warning" | "info" | "default" {
  switch (priority) {
    case "urgent": return "danger";
    case "high": return "warning";
    case "medium": return "info";
    case "low": return "default";
    default: return "default";
  }
}
</script>

<template>
  <div class="tickets">
    <!-- TICKET LIST -->
    <template v-if="!selectedId">
      <div class="tickets-header">
        <h1>Tickets</h1>
        <div class="tickets-stats">
          <span class="stat"><strong>{{ openCount }}</strong> open</span>
          <span class="stat"><strong>{{ pendingCount }}</strong> pending</span>
        </div>
      </div>

      <FilterBar :filters="filters" @update:filter="onFilterUpdate" />

      <div v-if="loading" class="loading">Loading...</div>

      <div v-else class="ticket-list">
        <div
          v-for="ticket in filteredTickets"
          :key="ticket.id"
          class="ticket-row"
          @click="selectTicket(ticket.id)"
        >
          <div class="ticket-main">
            <div class="ticket-title-line">
              <span class="ticket-title">{{ ticket.title }}</span>
              <AppBadge :label="ticket.priority" :variant="priorityVariant(ticket.priority)" />
            </div>
            <div class="ticket-meta">
              <span>{{ ticket.author_name }}</span>
              <span class="sep">in</span>
              <span>{{ ticket.server }}</span>
              <span class="sep">-</span>
              <span class="category">{{ ticket.category }}</span>
            </div>
          </div>
          <div class="ticket-side">
            <AppBadge :label="ticket.status" :variant="statusVariant(ticket.status)" />
            <span class="ticket-messages">{{ ticket.messages_count }} msg</span>
            <span class="ticket-date">{{ ticket.updated_at }}</span>
          </div>
        </div>

        <div v-if="filteredTickets.length === 0" class="empty">
          No tickets matching filters
        </div>
      </div>
    </template>

    <!-- TICKET DETAIL -->
    <template v-else>
      <div v-if="detailLoading" class="loading">Loading ticket...</div>

      <template v-else-if="detail">
        <div class="detail-header">
          <button class="back-btn" @click="backToList">&larr; Back</button>
          <div class="detail-title-row">
            <h1>{{ detail.ticket.title }}</h1>
            <div class="detail-badges">
              <AppBadge :label="detail.ticket.status" :variant="statusVariant(detail.ticket.status)" />
              <AppBadge :label="detail.ticket.priority" :variant="priorityVariant(detail.ticket.priority)" />
              <AppBadge :label="detail.ticket.category" variant="default" />
            </div>
          </div>
          <div class="detail-info">
            <span>By <strong>{{ detail.ticket.author_name }}</strong></span>
            <span class="sep">in</span>
            <span><strong>{{ detail.ticket.server }}</strong></span>
            <span class="sep">|</span>
            <span>Assigned to: <strong>{{ detail.ticket.assigned_to ?? "Unassigned" }}</strong></span>
            <span class="sep">|</span>
            <span>Created {{ detail.ticket.created_at }}</span>
          </div>
        </div>

        <!-- Messages / Conversation -->
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
              <span class="message-time">{{ msg.created_at }}</span>
            </div>
            <p class="message-content">{{ msg.content }}</p>
          </div>
        </div>

        <!-- Reply box -->
        <div v-if="detail.ticket.status !== 'closed'" class="reply-box">
          <textarea
            v-model="replyContent"
            placeholder="Type your reply..."
            rows="3"
            @keydown.ctrl.enter="sendReply"
          ></textarea>
          <div class="reply-actions">
            <AppButton variant="primary" :disabled="!replyContent.trim() || replying" @click="sendReply">
              {{ replying ? "Sending..." : "Reply" }}
            </AppButton>
            <AppButton variant="secondary" @click="closeTicket">
              Close ticket
            </AppButton>
          </div>
        </div>

        <div v-else class="closed-notice">
          This ticket is closed.
        </div>
      </template>
    </template>
  </div>
</template>

<style scoped>
.tickets-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 24px;
}

.tickets-header h1 {
  margin: 0;
}

.tickets-stats {
  display: flex;
  gap: 16px;
  color: var(--text-secondary);
  font-size: 13px;
}

.ticket-list {
  display: flex;
  flex-direction: column;
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  overflow: hidden;
}

.ticket-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border);
  cursor: pointer;
  transition: background-color 0.15s;
}

.ticket-row:last-child {
  border-bottom: none;
}

.ticket-row:hover {
  background-color: var(--bg-hover);
}

.ticket-main {
  display: flex;
  flex-direction: column;
  gap: 6px;
  flex: 1;
  min-width: 0;
}

.ticket-title-line {
  display: flex;
  align-items: center;
  gap: 10px;
}

.ticket-title {
  font-weight: 600;
  font-size: 14px;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
}

.ticket-meta {
  display: flex;
  gap: 6px;
  font-size: 12px;
  color: var(--text-secondary);
}

.ticket-meta .sep {
  opacity: 0.5;
}

.category {
  background-color: var(--bg-hover);
  padding: 1px 6px;
  border-radius: 3px;
}

.ticket-side {
  display: flex;
  align-items: center;
  gap: 12px;
  flex-shrink: 0;
  margin-left: 20px;
}

.ticket-messages {
  font-size: 12px;
  color: var(--text-secondary);
  white-space: nowrap;
}

.ticket-date {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  white-space: nowrap;
}

/* Detail view */
.detail-header {
  margin-bottom: 24px;
}

.back-btn {
  background: none;
  color: var(--text-secondary);
  padding: 4px 0;
  margin-bottom: 12px;
  font-size: 13px;
}

.back-btn:hover {
  color: var(--text-primary);
}

.detail-title-row {
  display: flex;
  align-items: center;
  gap: 12px;
  margin-bottom: 8px;
  flex-wrap: wrap;
}

.detail-title-row h1 {
  margin: 0;
  font-size: 22px;
}

.detail-badges {
  display: flex;
  gap: 6px;
}

.detail-info {
  font-size: 13px;
  color: var(--text-secondary);
  display: flex;
  gap: 6px;
  flex-wrap: wrap;
}

.detail-info .sep {
  opacity: 0.4;
}

/* Messages */
.messages-container {
  display: flex;
  flex-direction: column;
  gap: 12px;
  margin-bottom: 20px;
}

.message {
  padding: 16px;
  border-radius: 12px;
  max-width: 80%;
}

.message--user {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  align-self: flex-start;
}

.message--staff {
  background-color: rgba(88, 101, 242, 0.1);
  border: 1px solid rgba(88, 101, 242, 0.25);
  align-self: flex-end;
}

.message-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.message-author {
  font-weight: 600;
  font-size: 13px;
}

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

/* Reply box */
.reply-box {
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
  padding: 16px;
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
  transition: border-color 0.2s;
}

.reply-box textarea:focus {
  border-color: var(--accent);
}

.reply-actions {
  display: flex;
  gap: 8px;
  margin-top: 12px;
  justify-content: flex-end;
}

.reply-actions button.secondary {
  background-color: transparent;
  border: 1px solid var(--border);
  color: var(--text-secondary);
}

.reply-actions button.secondary:hover {
  border-color: var(--danger);
  color: var(--danger);
}

.closed-notice {
  text-align: center;
  padding: 20px;
  color: var(--text-secondary);
  background-color: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 12px;
}

.empty {
  text-align: center;
  color: var(--text-secondary);
  padding: 32px;
}
</style>
