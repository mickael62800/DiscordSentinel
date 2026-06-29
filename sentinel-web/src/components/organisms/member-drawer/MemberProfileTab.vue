<script setup lang="ts">
import type { Member, MemberStats, MemberInfractions, MemberModeration } from "../../../types";
import { formatMemberDate as formatDate, formatDuration, rolesCount } from "../../../utils/memberActivity";
import AppBadge from "../../atoms/AppBadge.vue";

defineProps<{
  member: Member;
  stats: MemberStats;
  infractions: MemberInfractions;
  moderation: MemberModeration;
}>();
</script>

<template>
  <div class="profile-meta">
    <div class="meta-item">
      <span class="meta-label">Membre depuis</span>
      <span class="meta-value">{{ formatDate(member.joined_at) }}</span>
    </div>
    <div class="meta-item">
      <span class="meta-label">Compte cree</span>
      <span class="meta-value">{{ formatDate(member.account_created) }}</span>
    </div>
    <div class="meta-item">
      <span class="meta-label">Roles</span>
      <span class="meta-value">{{ rolesCount(member.roles) }}</span>
    </div>
  </div>

  <div class="stats-row">
    <div class="stat-box">
      <span class="stat-number">{{ stats.message_count }}</span>
      <span class="stat-text">Messages</span>
    </div>
    <div class="stat-box">
      <span class="stat-number">{{ formatDuration(stats.voice_seconds) }}</span>
      <span class="stat-text">Vocal</span>
    </div>
    <div class="stat-box">
      <span class="stat-number">{{ infractions.total }}</span>
      <span class="stat-text">Infractions</span>
    </div>
    <div class="stat-box">
      <span class="stat-number stat-warn">{{ moderation.total_warns }}</span>
      <span class="stat-text">Warns</span>
    </div>
    <div class="stat-box">
      <span class="stat-number stat-mute">{{ moderation.total_mutes }}</span>
      <span class="stat-text">Mutes</span>
    </div>
    <div class="stat-box">
      <span class="stat-number stat-ban">{{ moderation.total_bans }}</span>
      <span class="stat-text">Bans</span>
    </div>
  </div>

  <div v-if="infractions.recent.length > 0" class="section">
    <h3>Infractions recentes</h3>
    <div v-for="(inf, i) in infractions.recent" :key="i" class="detail-row">
      <div class="detail-row-header">
        <span class="detail-date">{{ formatDate(inf.created_at) }}</span>
        <AppBadge :label="inf.action" variant="danger" />
      </div>
      <div class="detail-row-body">{{ inf.reason }}</div>
    </div>
  </div>
</template>

<style scoped>
.profile-meta {
  display: flex;
  gap: 24px;
  margin-bottom: 20px;
  padding: 12px 16px;
  background: var(--bg-secondary);
  border-radius: 8px;
}
.meta-item { display: flex; flex-direction: column; gap: 2px; }
.meta-label { font-size: 11px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.3px; }
.meta-value { font-size: 13px; font-weight: 600; color: var(--text-primary); }

.stats-row { display: flex; gap: 10px; margin-bottom: 20px; }
.stat-box {
  flex: 1;
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 12px 8px;
  text-align: center;
}
.stat-number { display: block; font-size: 18px; font-weight: 700; color: var(--text-primary); }
.stat-warn { color: var(--info) !important; }
.stat-mute { color: var(--warning) !important; }
.stat-ban { color: var(--danger) !important; }
.stat-text { font-size: 10px; color: var(--text-secondary); text-transform: uppercase; letter-spacing: 0.3px; }

.section { margin-bottom: 20px; }
.section h3 { margin: 0 0 10px 0; font-size: 14px; font-weight: 600; }

.detail-row {
  background: var(--bg-secondary);
  border-radius: 8px;
  padding: 10px 14px;
  margin-bottom: 6px;
}
.detail-row-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 8px;
  margin-bottom: 4px;
}
.detail-date {
  font-size: 11px;
  color: var(--text-secondary);
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  flex-shrink: 0;
}
.detail-row-body { font-size: 13px; color: var(--text-primary); white-space: pre-wrap; word-break: break-word; }
</style>
