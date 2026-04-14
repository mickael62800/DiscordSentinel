<script setup lang="ts">
import { computed } from "vue";

const props = defineProps<{
  details: Record<string, unknown>;
}>();

const hasDetails = computed(() => Object.keys(props.details).length > 0);
</script>

<template>
  <div v-if="hasDetails" class="entry-details">
    <!-- Message edit : avant / apres -->
    <template v-if="details.old_content !== undefined || details.new_content !== undefined">
      <div v-if="details.old_content" class="detail-block">
        <span class="detail-label">Avant :</span>
        <span class="detail-value detail-old">{{ details.old_content }}</span>
      </div>
      <div v-if="details.new_content" class="detail-block">
        <span class="detail-label">Apres :</span>
        <span class="detail-value detail-new">{{ details.new_content }}</span>
      </div>
    </template>

    <!-- Message delete : contenu supprime -->
    <template v-if="details.content && !details.new_content">
      <div class="detail-block">
        <span class="detail-label">Contenu :</span>
        <span class="detail-value">{{ details.content }}</span>
      </div>
      <div v-if="details.author_name" class="detail-block">
        <span class="detail-label">Auteur :</span>
        <span class="detail-value">{{ details.author_name }}</span>
      </div>
    </template>

    <!-- Message delete bulk -->
    <template v-if="details.count">
      <div class="detail-block">
        <span class="detail-label">Messages supprimes :</span>
        <span class="detail-value">{{ details.count }}</span>
      </div>
    </template>

    <!-- Role create / update -->
    <template v-if="details.colour || details.changes">
      <div v-if="details.colour" class="detail-block">
        <span class="detail-label">Couleur :</span>
        <span class="detail-value"><span class="color-dot" :style="{ backgroundColor: String(details.colour) }"></span> {{ details.colour }}</span>
      </div>
      <div v-if="details.position !== undefined" class="detail-block">
        <span class="detail-label">Position :</span>
        <span class="detail-value">{{ details.position }}</span>
      </div>
      <div v-if="details.mentionable !== undefined" class="detail-block">
        <span class="detail-label">Mentionnable :</span>
        <span class="detail-value">{{ details.mentionable ? 'Oui' : 'Non' }}</span>
      </div>
      <div v-if="details.hoist !== undefined" class="detail-block">
        <span class="detail-label">Affiche separement :</span>
        <span class="detail-value">{{ details.hoist ? 'Oui' : 'Non' }}</span>
      </div>
    </template>

    <!-- Role update changes list -->
    <template v-if="Array.isArray(details.changes)">
      <div v-for="(change, i) in (details.changes as string[])" :key="i" class="detail-block">
        <span class="detail-value mono">{{ change }}</span>
      </div>
    </template>

    <!-- Permission diff -->
    <div v-if="details.permission_diff" class="detail-block">
      <span class="detail-label">Permissions :</span>
      <pre class="detail-pre">{{ details.permission_diff }}</pre>
    </div>

    <!-- Channel create -->
    <template v-if="details.kind && !details.changes">
      <div class="detail-block">
        <span class="detail-label">Type :</span>
        <span class="detail-value">{{ details.kind }}</span>
      </div>
    </template>

    <!-- Roles changes (member_role_update) -->
    <template v-if="details.old_roles">
      <div class="detail-block">
        <span class="detail-label">Roles :</span>
        <span class="detail-value">{{ (details.old_roles as string[]).length }} → {{ (details.new_roles as string[]).length }}</span>
      </div>
    </template>

    <!-- Voice move -->
    <template v-if="details.from_channel || details.to_channel">
      <div class="detail-block">
        <span class="detail-label">Deplacement :</span>
        <span class="detail-value mono">{{ details.from_channel }} → {{ details.to_channel }}</span>
      </div>
    </template>

    <!-- Avatar change -->
    <template v-if="details.old_avatar_url || details.new_avatar_url">
      <div class="detail-avatars">
        <div v-if="details.old_avatar_url" class="avatar-block">
          <span class="detail-label">Avant :</span>
          <img :src="String(details.old_avatar_url)" class="avatar-preview" alt="Ancien avatar" />
        </div>
        <div v-if="details.old_avatar_url && details.new_avatar_url" class="avatar-arrow">→</div>
        <div v-if="details.new_avatar_url" class="avatar-block">
          <span class="detail-label">{{ details.old_avatar_url ? 'Apres :' : 'Nouvel avatar :' }}</span>
          <img :src="String(details.new_avatar_url)" class="avatar-preview" alt="Nouvel avatar" />
        </div>
      </div>
    </template>

    <!-- Member join : account age -->
    <template v-if="details.account_created_at">
      <div class="detail-block">
        <span class="detail-label">Compte cree le :</span>
        <span class="detail-value">{{ details.account_created_at }}</span>
      </div>
    </template>

    <!-- Anomaly -->
    <template v-if="details.anomaly_type">
      <div class="detail-block">
        <span class="detail-label">Type :</span>
        <span class="detail-value">{{ details.anomaly_type }}</span>
      </div>
      <div class="detail-block">
        <span class="detail-label">Nombre :</span>
        <span class="detail-value">{{ details.count }} en {{ details.window_secs }}s</span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.entry-details {
  margin-top: 8px;
  padding: 10px 12px;
  background-color: var(--bg-secondary);
  border-radius: 6px;
  font-size: 12px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.detail-block {
  display: flex;
  gap: 6px;
  align-items: baseline;
}

.detail-label {
  color: var(--text-secondary);
  white-space: nowrap;
  flex-shrink: 0;
}

.detail-value {
  color: var(--text-primary);
  word-break: break-word;
}

.detail-old {
  color: var(--danger);
  text-decoration: line-through;
  opacity: 0.8;
}

.detail-new {
  color: var(--success);
}

.detail-pre {
  margin: 0;
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
  font-size: 11px;
  color: var(--text-primary);
  white-space: pre-wrap;
  word-break: break-word;
}

.color-dot {
  display: inline-block;
  width: 10px;
  height: 10px;
  border-radius: 50%;
  vertical-align: middle;
  margin-right: 4px;
}

.mono {
  font-family: "JetBrains Mono", "Cascadia Code", monospace;
}

.detail-avatars {
  display: flex;
  align-items: center;
  gap: 12px;
}

.avatar-block {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 4px;
}

.avatar-preview {
  width: 64px;
  height: 64px;
  border-radius: 50%;
  border: 2px solid var(--border);
  object-fit: cover;
}

.avatar-arrow {
  font-size: 20px;
  color: var(--text-secondary);
  font-weight: 700;
}
</style>
