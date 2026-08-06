<script setup lang="ts">
// Faire poster au bot un message texte dans un salon.
//
// Le pendant dépouillé du builder d'embeds : ici il n'y a rien à construire,
// seulement du markdown à écrire. C'est ce qu'on veut quand le message doit
// ressembler à celui d'un membre plutôt qu'à une notification de service.
//
// Rien n'est enregistré. Un message envoyé appartient à Discord : en garder
// une copie ici créerait deux vérités, dont l'une serait fausse dès la
// première édition manuelle dans Discord.

import { computed, ref, watch } from "vue";

import AdminPageShell from "../layouts/AdminPageShell.vue";
import ActionButton from "../atoms/ActionButton.vue";
import { useGuildSelector } from "../../composables/useGuildSelector";
import { guildChannelsService } from "@/services/guildChannelsService";
import { messagesService, MAX_MESSAGE_LENGTH } from "@/services/messagesService";
import type { DiscordChannelInfo } from "@/types";

const { selectedGuildId, selectedGuild } = useGuildSelector();

const channels = ref<DiscordChannelInfo[]>([]);
const channelId = ref("");
const content = ref("");
const sending = ref(false);
const errorMessage = ref("");
const successMessage = ref("");

const longueur = computed(() => [...content.value].length);
const tropLong = computed(() => longueur.value > MAX_MESSAGE_LENGTH);

/// Un ping de masse ne doit pas partir sur un clic distrait : on le signale
/// avant, et on le confirme au moment d'envoyer.
const pingeTout = computed(() => /@everyone|@here/.test(content.value));

const peutEnvoyer = computed(
  () =>
    !!selectedGuildId.value &&
    !!channelId.value &&
    content.value.trim().length > 0 &&
    !tropLong.value &&
    !sending.value,
);

async function chargerSalons() {
  channels.value = [];
  channelId.value = "";
  if (!selectedGuildId.value) return;
  try {
    channels.value = await guildChannelsService.listTextChannels(selectedGuildId.value);
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Salons illisibles";
  }
}

async function envoyer() {
  if (!peutEnvoyer.value || !selectedGuildId.value) return;

  if (pingeTout.value) {
    const salon = channels.value.find((c) => c.id === channelId.value)?.name ?? "ce salon";
    const ok = window.confirm(
      `Ce message mentionne @everyone ou @here dans #${salon}.\n\nTout le serveur sera notifié. Confirmer ?`,
    );
    if (!ok) return;
  }

  sending.value = true;
  errorMessage.value = "";
  successMessage.value = "";
  try {
    await messagesService.send(selectedGuildId.value, channelId.value, content.value);
    // « Transmis » et non « envoyé » : l'API a mis l'ordre en file, le bot
    // poste ensuite. Promettre un envoi réussi mentirait si le bot n'a pas
    // accès au salon.
    successMessage.value = "Message transmis au bot.";
    content.value = "";
  } catch (e) {
    errorMessage.value = e instanceof Error ? e.message : "Envoi impossible";
  } finally {
    sending.value = false;
  }
}

watch(selectedGuildId, chargerSalons, { immediate: true });
</script>

<template>
  <AdminPageShell
    title="Envoyer un message"
    :subtitle="selectedGuild?.name ?? 'Aucun serveur sélectionné'"
  >
    <p v-if="!selectedGuildId" class="sm-hint">
      Sélectionne un serveur Discord pour écrire un message.
    </p>

    <template v-else>
      <label class="sm-label" for="sm-salon">Salon</label>
      <select id="sm-salon" v-model="channelId" class="sm-select">
        <option value="" disabled>Choisis un salon…</option>
        <option v-for="c in channels" :key="c.id" :value="c.id">#{{ c.name }}</option>
      </select>

      <label class="sm-label" for="sm-texte">Message</label>
      <textarea
        id="sm-texte"
        v-model="content"
        class="sm-textarea"
        rows="10"
        placeholder="Le markdown Discord fonctionne : **gras**, *italique*, `code`, > citation, [lien](url), <@&role>…"
      ></textarea>

      <div class="sm-barre">
        <span :class="['sm-compteur', tropLong && 'sm-trop']">
          {{ longueur }} / {{ MAX_MESSAGE_LENGTH }}
        </span>
        <ActionButton :disabled="!peutEnvoyer" @click="envoyer">
          {{ sending ? "Envoi…" : "Envoyer" }}
        </ActionButton>
      </div>

      <p v-if="pingeTout" class="sm-avertissement">
        ⚠️ Ce message mentionne <code>@everyone</code> ou <code>@here</code> : tout le
        serveur sera notifié.
      </p>
      <p v-if="tropLong" class="sm-error">
        Discord refuse les messages de plus de {{ MAX_MESSAGE_LENGTH }} caractères.
      </p>
      <p v-if="errorMessage" class="sm-error">{{ errorMessage }}</p>
      <p v-if="successMessage" class="sm-ok">{{ successMessage }}</p>

      <p class="sm-note">
        Le message est posté par le bot, sans encadré : il ressemble à un message
        normal. Il n'est pas conservé ici — pour le modifier ou le supprimer, il faut
        passer par Discord.
      </p>
    </template>
  </AdminPageShell>
</template>

<style scoped>
.sm-hint,
.sm-note {
  color: var(--text-secondary);
}

.sm-note {
  margin-top: var(--space-md);
  font-size: 0.86rem;
}

.sm-label {
  display: block;
  margin: var(--space-md) 0 var(--space-xs);
  color: var(--text-secondary);
  font-weight: 600;
}

.sm-select,
.sm-textarea {
  width: 100%;
  padding: var(--space-xs) var(--space-sm);
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-md);
  color: var(--text-primary);
  font-family: inherit;
}

.sm-select {
  max-width: 22rem;
}

.sm-textarea {
  resize: vertical;
  line-height: 1.5;
}

.sm-select:focus,
.sm-textarea:focus {
  outline: none;
  border-color: var(--accent);
}

.sm-barre {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: var(--space-md);
  margin-top: var(--space-sm);
}

.sm-compteur {
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.sm-trop,
.sm-error {
  color: var(--danger);
}

.sm-avertissement {
  color: var(--warning, #e67e22);
}

.sm-ok {
  color: var(--success);
}
</style>
