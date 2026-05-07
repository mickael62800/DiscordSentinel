<script setup lang="ts">
import AppInput from "@/components/atoms/AppInput.vue";
import { onMounted, reactive, ref, watch } from "vue";
import { useGuildSelector } from "@/composables/useGuildSelector";
import { useToast } from "@/composables/useToast";
import { sponsorshipsService } from "@/services/polishServices";
import type { Sponsorship } from "@/types/polish";
import AdminPageShell from "@/components/layouts/AdminPageShell.vue";

const { guildIdFilter } = useGuildSelector();
const { success, error: showError } = useToast();
const sponsorships = ref<Sponsorship[]>([]);
const loading = ref(true);

const draft = reactive({
  sponsor_id: "",
  sponsored_id: "",
});

async function fetchSponsorships() {
  if (!guildIdFilter.value) {
    sponsorships.value = [];
    loading.value = false;
    return;
  }
  loading.value = true;
  try {
    sponsorships.value = await sponsorshipsService.list(guildIdFilter.value);
  } catch (e) {
    console.error(e);
    showError("Erreur chargement parrainages.");
  } finally {
    loading.value = false;
  }
}

async function onCreate() {
  if (!guildIdFilter.value) return;
  if (!draft.sponsor_id.trim() || !draft.sponsored_id.trim()) {
    showError("Sponsor + filleul requis.");
    return;
  }
  try {
    await sponsorshipsService.create({
      guild_id: guildIdFilter.value,
      sponsor_id: draft.sponsor_id.trim(),
      sponsored_id: draft.sponsored_id.trim(),
    });
    draft.sponsor_id = "";
    draft.sponsored_id = "";
    success("Parrainage enregistré.");
    await fetchSponsorships();
  } catch (e) {
    console.error(e);
    showError("Erreur création parrainage.");
  }
}

onMounted(fetchSponsorships);
watch(guildIdFilter, fetchSponsorships);

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString("fr-FR");
}
</script>

<template>
  <AdminPageShell title="Parrainages" icon="🤝">
    <template #lede>
      Liens parrain → filleul enregistrés sur le serveur. Le système de
      parrainage récompense le parrain quand son filleul atteint le niveau
      configuré (cf. <code>community-bot</code> config).
    </template>

    <section class="card">
      <h2>Nouveau parrainage</h2>
      <form @submit.prevent="onCreate" class="form">
        <label>
          Sponsor (parrain) ID *
          <AppInput v-model="draft.sponsor_id" required />
        </label>
        <label>
          Sponsored (filleul) ID *
          <AppInput v-model="draft.sponsored_id" required />
        </label>
        <div class="actions full">
          <button type="submit" class="btn-primary">Enregistrer</button>
        </div>
      </form>
    </section>

    <section class="card">
      <h2>Parrainages actifs ({{ sponsorships.length }})</h2>
      <div v-if="loading" class="loading">Chargement…</div>
      <div v-else-if="sponsorships.length === 0" class="empty">
        Aucun parrainage enregistré.
      </div>
      <table v-else class="table">
        <thead>
          <tr>
            <th>Date</th>
            <th>Sponsor</th>
            <th>Filleul</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="s in sponsorships" :key="s.id">
            <td>{{ formatDate(s.created_at) }}</td>
            <td><code>{{ s.sponsor_id }}</code></td>
            <td><code>{{ s.sponsored_id }}</code></td>
          </tr>
        </tbody>
      </table>
    </section>
  </AdminPageShell>
</template>

<style scoped>
@import "./_admin-page-shared.css";
.form {
  display: grid;
  grid-template-columns: 1fr;
  gap: 12px;
}
.form label {
  display: flex;
  flex-direction: column;
  gap: 4px;
}
.form input {
  background: var(--bg-card);
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px 12px;
  color: inherit;
  font-size: 13px;
}
.actions.full {
  justify-content: flex-end;
}
</style>
