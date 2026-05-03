<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { remindersService } from "@/services/moderationAdvancedService";
import type { SanctionReminder } from "@/types/moderation-advanced";

const { guildIdFilter } = useGuildSelector();
const { error: showError } = useToast();
const reminders = ref<SanctionReminder[]>([]);
const loading = ref(true);

async function fetchReminders() {
  if (!guildIdFilter.value) {
    reminders.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    reminders.value = await remindersService.listByGuild(guildIdFilter.value);
  } catch (e) {
    console.error(e);
    showError("Erreur chargement reminders.");
  } finally {
    loading.value = false;
  }
}

onMounted(fetchReminders);
watch(guildIdFilter, fetchReminders);

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR");
}
function statusColor(s: string): string {
  if (s === "pending") return "#F1C40F";
  if (s === "sent") return "#2ECC71";
  if (s === "expired") return "#7F8C8D";
  return "#888";
}
</script>

<template>
  <div class="page page--wide">
    <header class="page-header">
      <h1>⏰ Reminders modération</h1>
      <p class="lede">
        Rappels datés sur les actions de modération (ex. notification quand un
        mute temporaire approche de son terme). Créés automatiquement par les
        commandes <code>/mute duration</code> et <code>/ban duration</code>,
        consultables ici.
      </p>
    </header>

    <section class="card">
      <h2>Reminders du serveur</h2>
      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="reminders.length === 0" class="empty">
        Aucun reminder enregistré.
      </div>
      <table v-else class="table">
        <thead>
          <tr>
            <th>Cible</th>
            <th>Action</th>
            <th>Modérateur</th>
            <th>Rappel</th>
            <th>Expire</th>
            <th>Statut</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="r in reminders" :key="r.id">
            <td>
              <strong>{{ r.target_name }}</strong>
              <small class="muted">{{ r.target_id }}</small>
            </td>
            <td><code>{{ r.action_type }}</code></td>
            <td>{{ r.moderator_name }}</td>
            <td>{{ formatDate(r.remind_at) }}</td>
            <td>{{ formatDate(r.expires_at) }}</td>
            <td>
              <span class="badge" :style="{ backgroundColor: statusColor(r.status) }">
                {{ r.status }}
              </span>
            </td>
          </tr>
        </tbody>
      </table>
    </section>
  </div>
</template>

<style scoped>
@import "./_moderation-advanced-shared.css";
</style>
