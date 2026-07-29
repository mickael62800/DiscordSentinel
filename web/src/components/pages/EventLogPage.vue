<script setup lang="ts">
// Journal d'evenements — remplace les salons de logs Discord.
//
// Tout ce que le bot postait dans #logs-membres, #logs-vocal, #logs-messages,
// #logs-profils et #commandes-admin se consulte ici.
//
// Choix structurants :
//   - filtrage et pagination cote SERVEUR. La page Audit historique chargeait
//     500 entrees puis filtrait en memoire ; comme seule vue des evenements,
//     ca ne tient pas sur la duree de retention.
//   - vues par NATURE d'evenement (une par ancien salon) plutot qu'une
//     timeline unique, pour retrouver le confort des salons dedies.
//   - temps reel : les nouveaux evenements sont inseres en tete sans recharger
//     toute la page, et uniquement sur la premiere page (sinon on deplacerait
//     le contenu sous les yeux de quelqu'un en train de lire l'historique).

import { computed, onMounted, onUnmounted, ref, watch } from "vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { useRealtime } from "../../composables/useRealtime";
import {
  EVENT_CATEGORIES,
  eventLogService,
  type EventCategory,
} from "@/services/eventLogService";
import { eventIcon, eventLabel, eventVariant } from "@/utils/variants";
import type { AuditLog } from "@/types";
import AdminPageShell from "../layouts/AdminPageShell.vue";

const PAGE_SIZE = 50;

const { selectedGuildId, selectedGuild } = useGuildSelector();

const category = ref<EventCategory>(EVENT_CATEGORIES[0]);
const search = ref("");
const from = ref("");
const to = ref("");
const page = ref(0);

const entries = ref<AuditLog[]>([]);
const total = ref(0);
const loading = ref(false);
const errorMessage = ref("");
/// Compteur d'evenements arrives pendant qu'on consulte une page > 1.
const pendingLive = ref(0);

const totalPages = computed(() => Math.max(1, Math.ceil(total.value / PAGE_SIZE)));

/// Convertit une date de champ `date` (AAAA-MM-JJ) en RFC3339. `endOfDay`
/// inclut la journee entiere, sinon un filtre "au 5 mars" exclurait le 5.
function toRfc(value: string, endOfDay = false): string | null {
  if (!value) return null;
  const d = new Date(value + (endOfDay ? "T23:59:59.999" : "T00:00:00"));
  return Number.isNaN(d.getTime()) ? null : d.toISOString();
}

async function load() {
  if (!selectedGuildId.value) {
    entries.value = [];
    total.value = 0;
    return;
  }
  loading.value = true;
  errorMessage.value = "";
  try {
    const res = await eventLogService.list({
      guildId: selectedGuildId.value,
      eventTypes: category.value.eventTypes,
      from: toRfc(from.value),
      to: toRfc(to.value, true),
      search: search.value.trim() || null,
      limit: PAGE_SIZE,
      offset: page.value * PAGE_SIZE,
    });
    entries.value = res.data;
    total.value = res.total;
    if (page.value === 0) pendingLive.value = 0;
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Chargement impossible";
    entries.value = [];
  } finally {
    loading.value = false;
  }
}

/// Un filtre modifie remet a la premiere page : rester page 7 d'un resultat
/// qui n'en compte plus que 2 afficherait un vide inexplicable.
function resetAndLoad() {
  page.value = 0;
  load();
}

let debounce: ReturnType<typeof setTimeout> | null = null;
watch(search, () => {
  if (debounce) clearTimeout(debounce);
  debounce = setTimeout(resetAndLoad, 350);
});
watch([category, from, to], resetAndLoad);
watch(selectedGuildId, resetAndLoad, { immediate: true });
watch(page, load);

/// Insertion temps reel. On ne recharge pas : un refetch complet a chaque
/// evenement rendrait la page illisible sur un serveur actif.
function onLiveEvent(payload: unknown) {
  const log = payload as AuditLog | undefined;
  if (!log?.id || log.guild_id !== selectedGuildId.value) return;

  const types = category.value.eventTypes;
  if (types.length && !types.includes(log.event_type)) return;

  total.value += 1;
  if (page.value !== 0) {
    pendingLive.value += 1;
    return;
  }
  entries.value = [log, ...entries.value].slice(0, PAGE_SIZE);
}

const { onEvent } = useRealtime();
let unlisten: (() => void) | null = null;

onMounted(async () => {
  const fn = await onEvent("audit_log_created", onLiveEvent);
  unlisten = fn as unknown as () => void;
});
onUnmounted(() => unlisten?.());

function fmtDate(iso: string): string {
  const d = new Date(iso);
  return Number.isNaN(d.getTime()) ? iso : d.toLocaleString("fr-FR");
}

/// Resume lisible d'un evenement, sans afficher le JSON brut.
function summary(log: AuditLog): string {
  const d = log.details ?? {};
  const fromName = d.from_channel_name as string | undefined;
  const toName = d.to_channel_name as string | undefined;
  switch (log.event_type) {
    case "voice_join":
      return toName ? `a rejoint ${toName}` : "a rejoint un salon vocal";
    case "voice_leave":
      return fromName ? `a quitte ${fromName}` : "a quitte un salon vocal";
    case "voice_move":
      return fromName && toName ? `${fromName} → ${toName}` : "a change de salon";
    case "admin_command":
      return `/${(d.command as string) ?? "?"}${d.reason ? ` — ${d.reason}` : ""}`;
    default:
      return log.channel_name ? `dans ${log.channel_name}` : "";
  }
}
</script>

<template>
  <AdminPageShell
    title="Journal des evenements"
    :subtitle="selectedGuild?.name ?? 'Aucun serveur selectionne'"
  >
    <p v-if="!selectedGuildId" class="el-hint">
      Selectionne un serveur Discord pour consulter son journal.
    </p>

    <template v-else>
      <div class="el-tabs">
        <button
          v-for="c in EVENT_CATEGORIES"
          :key="c.key"
          type="button"
          class="el-tab"
          :class="{ active: c.key === category.key }"
          @click="category = c"
        >
          {{ c.label }}
        </button>
      </div>

      <div class="el-filters">
        <input v-model="search" type="search" placeholder="Rechercher un membre, un salon…" />
        <label>Du <input v-model="from" type="date" /></label>
        <label>Au <input v-model="to" type="date" /></label>
        <button v-if="search || from || to" type="button" class="el-reset" @click="search = ''; from = ''; to = ''">
          Effacer
        </button>
      </div>

      <p v-if="errorMessage" class="el-error">{{ errorMessage }}</p>

      <p v-if="pendingLive" class="el-live" @click="page = 0">
        {{ pendingLive }} nouvel(s) evenement(s) — revenir au debut
      </p>

      <p v-if="loading" class="el-hint">Chargement…</p>

      <p v-else-if="!entries.length" class="el-hint">Aucun evenement pour ces criteres.</p>

      <ul v-else class="el-list">
        <li v-for="e in entries" :key="e.id" class="el-item">
          <span class="el-badge" :class="`v-${eventVariant(e.event_type)}`">
            {{ eventIcon(e.event_type) }}
          </span>
          <div class="el-body">
            <div class="el-line">
              <strong>{{ eventLabel(e.event_type) }}</strong>
              <span v-if="e.actor_name" class="el-actor">{{ e.actor_name }}</span>
              <span v-if="e.target_name" class="el-target">→ {{ e.target_name }}</span>
            </div>
            <div v-if="summary(e)" class="el-summary">{{ summary(e) }}</div>
          </div>
          <time class="el-date">{{ fmtDate(e.created_at) }}</time>
        </li>
      </ul>

      <div v-if="total > PAGE_SIZE" class="el-pager">
        <button type="button" :disabled="page === 0" @click="page--">Precedent</button>
        <span>Page {{ page + 1 }} / {{ totalPages }} — {{ total }} evenement(s)</span>
        <button type="button" :disabled="page + 1 >= totalPages" @click="page++">Suivant</button>
      </div>
    </template>
  </AdminPageShell>
</template>

<style scoped>
.el-hint {
  color: var(--text-secondary);
}

.el-error {
  color: var(--danger);
}

.el-tabs {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-xs);
  margin-bottom: var(--space-md);
}

.el-tab {
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  color: var(--text-secondary);
  border-radius: var(--radius-md);
  padding: 4px 12px;
  cursor: pointer;
  transition: var(--transition-fast);
}

.el-tab:hover {
  color: var(--text-primary);
}

.el-tab.active {
  border-color: var(--accent);
  color: var(--text-primary);
}

.el-filters {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: var(--space-sm);
  margin-bottom: var(--space-md);
  color: var(--text-secondary);
  font-size: 0.88rem;
}

.el-filters input {
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  padding: 4px 8px;
}

.el-filters input[type="search"] {
  min-width: 16rem;
}

.el-reset {
  background: none;
  border: none;
  color: var(--text-secondary);
  text-decoration: underline;
  cursor: pointer;
}

.el-live {
  background: color-mix(in srgb, var(--accent) 15%, transparent);
  color: var(--text-primary);
  padding: var(--space-xs) var(--space-sm);
  border-radius: var(--radius-md);
  cursor: pointer;
}

.el-list {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
}

.el-item {
  display: flex;
  align-items: flex-start;
  gap: var(--space-sm);
  padding: var(--space-sm);
  border-bottom: 1px solid var(--bg-hover);
}

.el-badge {
  flex: 0 0 1.6rem;
  height: 1.6rem;
  display: grid;
  place-items: center;
  border-radius: 50%;
  background: var(--bg-hover);
  color: var(--text-secondary);
  font-size: 0.78rem;
  font-weight: 700;
}

.v-danger {
  background: color-mix(in srgb, var(--danger) 25%, transparent);
  color: var(--danger);
}

.v-success {
  background: color-mix(in srgb, var(--success) 25%, transparent);
  color: var(--success);
}

.v-warning {
  background: color-mix(in srgb, var(--warning) 25%, transparent);
  color: var(--warning);
}

.el-body {
  flex: 1;
  min-width: 0;
}

.el-line {
  display: flex;
  flex-wrap: wrap;
  gap: var(--space-xs);
  align-items: baseline;
}

.el-actor {
  color: var(--text-primary);
}

.el-target,
.el-summary {
  color: var(--text-secondary);
  font-size: 0.88rem;
}

.el-date {
  color: var(--text-secondary);
  font-size: 0.82rem;
  white-space: nowrap;
}

.el-pager {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: var(--space-md);
  margin-top: var(--space-md);
  color: var(--text-secondary);
  font-size: 0.88rem;
}

.el-pager button {
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  color: var(--text-primary);
  border-radius: var(--radius-sm);
  padding: 4px 12px;
  cursor: pointer;
}

.el-pager button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

@media (max-width: 700px) {
  .el-item {
    flex-wrap: wrap;
  }

  .el-date {
    width: 100%;
    padding-left: 2.2rem;
  }
}
</style>
