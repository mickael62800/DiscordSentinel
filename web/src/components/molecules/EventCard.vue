<script setup lang="ts">
// Carte d'événement, calquée sur les embeds Discord.
//
// Les modules postaient leurs cartes dans un salon Discord : barre de couleur
// à gauche, emoji, titre, puis une ou deux lignes de détail. On reprend la
// même grammaire visuelle — y compris la COULEUR envoyée par le bot — pour
// qu'un modérateur retrouve ici ce qu'il lisait là-bas.
//
// La couleur vient des données, pas d'une table locale : le bot reste la
// source unique du code couleur d'une sanction. Une table dupliquée ici
// dériverait au premier ajout de type.

import { computed } from "vue";
import type { AuditLog } from "@/types";

const props = defineProps<{ log: AuditLog }>();

type Details = Record<string, unknown>;
const d = computed<Details>(() => (props.log.details ?? {}) as Details);

function texte(cle: string): string {
  const v = d.value[cle];
  return typeof v === "string" ? v : "";
}

/// Couleur Discord (entier 24 bits) convertie en CSS. Repli neutre si absente.
const couleur = computed(() => {
  const c = d.value.color;
  if (typeof c !== "number" || c < 0 || c > 0xffffff) return "var(--border)";
  return `#${c.toString(16).padStart(6, "0")}`;
});

const emoji = computed(() => texte("emoji") || "•");
const action = computed(() => texte("action") || props.log.event_type);
const raison = computed(() => texte("reason"));
const duree = computed(() => texte("duration"));
const acteur = computed(() => texte("actor"));

const horodatage = computed(() =>
  new Date(props.log.created_at).toLocaleString("fr-FR", {
    dateStyle: "short",
    timeStyle: "short",
  }),
);
</script>

<template>
  <article class="ec" :style="{ '--barre': couleur }">
    <div class="ec-corps">
      <div class="ec-titre">
        <span class="ec-emoji" aria-hidden="true">{{ emoji }}</span>
        <strong>{{ action }}</strong>
        <span v-if="log.target_name" class="ec-cible">{{ log.target_name }}</span>
        <span v-if="log.target_id" class="ec-id">{{ log.target_id }}</span>
        <time class="ec-date" :datetime="log.created_at">{{ horodatage }}</time>
      </div>

      <p class="ec-ligne">
        <span v-if="acteur">Par <strong>{{ acteur }}</strong></span>
        <span v-if="raison" class="ec-sep">· Raison : {{ raison }}</span>
        <span v-if="duree" class="ec-sep">· Durée : {{ duree }}</span>
      </p>
    </div>
  </article>
</template>

<style scoped>
/* La barre colorée à gauche reprend le liseré des embeds Discord. */
.ec {
  display: flex;
  background: var(--bg-secondary);
  border: 1px solid var(--border);
  border-left: 4px solid var(--barre);
  border-radius: var(--radius-sm);
  padding: 10px 14px;
}

.ec-corps { flex: 1; min-width: 0; }

.ec-titre {
  display: flex;
  align-items: baseline;
  flex-wrap: wrap;
  gap: 8px;
  color: var(--text-primary);
  font-size: 14px;
}

.ec-emoji { font-size: 15px; }
.ec-cible { color: var(--text-primary); }

.ec-id {
  font-size: 11px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.ec-date {
  margin-left: auto;
  font-size: 12px;
  color: var(--text-secondary);
  font-variant-numeric: tabular-nums;
}

.ec-ligne {
  margin-top: 4px;
  font-size: 13px;
  color: var(--text-secondary);
  line-height: 1.5;
  overflow-wrap: anywhere;
}

.ec-sep { margin-left: 6px; }
</style>
