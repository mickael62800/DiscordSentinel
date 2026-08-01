<script setup lang="ts">
// En-tête des pages du site public : le logo, une phrase, des actions.
//
// Les trois pages publiques ouvraient sur la même composition, recopiée trois
// fois avec des tailles de logo légèrement différentes — assez pour qu'on
// sente le décalage en passant de l'une à l'autre sans savoir le nommer.
//
// Le wordmark est une illustration complète, avec son propre décor : il occupe
// donc seul la largeur, jamais superposé à une photo. L'essai inverse donnait
// l'impression de deux images qui se coupent.

import { COMMUNITY, onWordmarkError, wordmarkOf } from "@/branding";

defineProps<{
  /// Titre de la page, sous le logo. Absent sur l'accueil, où le wordmark
  /// EST le titre : l'y répéter ferait doublon.
  titre?: string;
  /// Phrase sous le logo. Absente = pas de ligne vide.
  tagline?: string;
  /// `grand` pour l'accueil, `compact` pour les pages intérieures : le même
  /// logo pleine taille partout écraserait le contenu des pages secondaires.
  taille?: "grand" | "compact";
}>();
</script>

<template>
  <header class="hero" :class="taille ?? 'grand'">
    <img
      class="hero-logo"
      :src="wordmarkOf(COMMUNITY)"
      :alt="COMMUNITY.name"
      @error="onWordmarkError($event, COMMUNITY)"
    />

    <h1 v-if="titre" class="hero-titre">{{ titre }}</h1>

    <p v-if="tagline" class="hero-tagline">{{ tagline }}</p>

    <!-- Chiffres, puces d'état : ce qui varie d'une page à l'autre. -->
    <div v-if="$slots.info" class="hero-info">
      <slot name="info" />
    </div>

    <div v-if="$slots.actions" class="hero-actions">
      <slot name="actions" />
    </div>
  </header>
</template>

<style scoped>
.hero {
  text-align: center;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-lg);
}

/* Le titre reste collé au logo : ensemble ils forment l'identité de la page,
   séparés ils se liraient comme deux blocs sans rapport. */
.hero-titre + .hero-tagline {
  margin-top: calc(-1 * var(--space-sm));
}

.hero-logo {
  display: block;
  height: auto;
  /* Le halo prolonge le néon du logo au lieu d'ajouter une ombre portée, qui
     l'aurait posé « sur » la page comme un autocollant. */
  filter: drop-shadow(0 10px 40px rgba(168, 85, 247, 0.35));
}

.hero.grand .hero-logo {
  width: min(420px, 76vw);
}

.hero.compact .hero-logo {
  width: min(200px, 44vw);
}

.hero-titre {
  margin: 0;
  font-size: clamp(1.4rem, 4vw, 2rem);
  line-height: 1.2;
  text-wrap: balance;
}

.hero-tagline {
  margin: 0;
  max-width: 44rem;
  color: var(--text-secondary);
  font-size: 1.05rem;
  line-height: 1.6;
  text-wrap: balance;
}

.hero.compact .hero-tagline {
  font-size: 0.95rem;
}

.hero-info {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: center;
  gap: var(--space-sm);
}

.hero-actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: var(--space-md);
}
</style>
