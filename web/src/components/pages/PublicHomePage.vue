<script setup lang="ts">
// Accueil PUBLIC du site communautaire — visible sans connexion.
//
// C'est la vitrine de La Bande du Canapé : un visiteur doit comprendre en
// quelques secondes ce qu'est la communauté et avoir envie d'entrer. Rendue
// hors de `MainLayout` (ni barre latérale, ni sélecteur de serveur) : c'est
// une page publique, pas un back-office.
//
// Elle n'appelle QUE `/api/public/*`, qui vit en dehors de la pile
// d'authentification. Aucun token n'est envoyé, aucune donnée personnelle
// n'est demandée.
//
// Direction artistique reprise du logo : violet néon sur fond sombre. Les
// dégradés et halos sont en CSS pur — aucune image de fond à télécharger.

import { onMounted, ref } from "vue";
import {
  guildIconUrl,
  publicSiteService,
  type PublicGuild,
} from "@/services/publicSiteService";
import { COMMUNITY, onWordmarkError, wordmarkOf } from "@/branding";

const guildId = import.meta.env.VITE_PUBLIC_GUILD_ID as string | undefined;

const guild = ref<PublicGuild | null>(null);
const iconUrl = ref<string | null>(null);

onMounted(async () => {
  if (!guildId) return;
  try {
    const g = await publicSiteService.guild(guildId);
    guild.value = g;
    iconUrl.value = guildIconUrl(g);
  } catch {
    // Vitrine indisponible : on n'affiche pas d'erreur technique à un
    // visiteur, la page garde tout son sens sans ce bloc.
    guild.value = null;
  }
});

/// Ce qu'on trouve sur le serveur. Volontairement concret : un visiteur veut
/// savoir ce qu'il va y faire, pas lire une charte.
const PILIERS = [
  {
    emoji: "🎮",
    titre: "On joue ensemble",
    texte:
      "Serveurs de jeu montés à la demande, sessions organisées, et de quoi trouver du monde à toute heure.",
  },
  {
    emoji: "🛋️",
    titre: "On se pose",
    texte:
      "Des salons vocaux ouverts en permanence. Personne n'est obligé de parler : venir écouter, ça compte aussi.",
  },
  {
    emoji: "🏆",
    titre: "On se chambre",
    texte:
      "Classements, concours, petits jeux internes et une économie maison pour pimenter tout ça.",
  },
  {
    emoji: "🤝",
    titre: "On veille",
    texte:
      "Une modération présente et transparente, pour que le canapé reste confortable pour tout le monde.",
  },
];
</script>

<template>
  <div class="ph">
    <!-- Halos décoratifs : reprennent le néon violet du logo. -->
    <div class="ph-glow ph-glow--1" aria-hidden="true"></div>
    <div class="ph-glow ph-glow--2" aria-hidden="true"></div>

    <header class="ph-hero">
      <img
        :src="wordmarkOf(COMMUNITY)"
        :alt="COMMUNITY.name"
        class="ph-logo"
        @error="onWordmarkError($event, COMMUNITY)"
      />

      <p class="ph-tagline">{{ COMMUNITY.tagline }}</p>

      <div v-if="guild" class="ph-stats">
        <img v-if="iconUrl" :src="iconUrl" :alt="guild.name" class="ph-guild-icon" />
        <span>
          <strong>{{ guild.member_count.toLocaleString("fr-FR") }}</strong>
          membres sur {{ guild.name }}
        </span>
      </div>

      <div class="ph-actions">
        <RouterLink class="ph-btn primary" to="/login">Rejoindre la bande</RouterLink>
      </div>
    </header>

    <section class="ph-about">
      <h2>C'est quoi, La Bande du Canapé&nbsp;?</h2>
      <p>
        Un serveur Discord sans prise de tête. On s'y retrouve pour jouer, pour
        discuter, ou juste pour avoir un peu de bruit de fond pendant qu'on fait
        autre chose. Pas de niveau minimum, pas d'audition&nbsp;: si tu poses tes
        fesses sur le canapé, tu fais partie de la bande.
      </p>
    </section>

    <section class="ph-piliers">
      <article v-for="p in PILIERS" :key="p.titre" class="ph-card">
        <span class="ph-card-emoji" aria-hidden="true">{{ p.emoji }}</span>
        <h3>{{ p.titre }}</h3>
        <p>{{ p.texte }}</p>
      </article>
    </section>

    <footer class="ph-footer">
      <RouterLink to="/login">Espace membre et administration</RouterLink>
    </footer>
  </div>
</template>

<style scoped>
/* Palette locale calée sur le logo (violet néon sur fond très sombre). Elle ne
   dépend pas du thème du back-office : cette page a sa propre identité. */
.ph {
  /* `#app` est un conteneur flex : sans `flex: 1`, la page ne prendrait que la
     largeur de son contenu et tout se tasserait à gauche. */
  flex: 1;
  position: relative;
  /* `overflow-x: hidden` : les halos décoratifs débordent volontairement des
     bords (left/right négatifs) et créaient une barre de défilement
     horizontale. `overflow-y: auto` sert de filet sur les très petits écrans —
     aux tailles courantes le contenu tient sans défilement. */
  overflow-x: hidden;
  overflow-y: auto;
  /* Tout est dimensionné en `vh` pour tenir sur une seule page. */
  padding: clamp(1rem, 3vh, 2.5rem) 1.5rem clamp(1rem, 2vh, 2rem);
  background: radial-gradient(circle at 50% 0%, #241040 0%, #120821 45%, #0a0512 100%);
  color: #f3eaff;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: clamp(1rem, 3vh, 2.25rem);
}

.ph-glow {
  position: absolute;
  border-radius: 50%;
  filter: blur(90px);
  opacity: 0.5;
  pointer-events: none;
}

.ph-glow--1 {
  width: 30rem;
  height: 30rem;
  top: -8rem;
  left: -6rem;
  background: #7c3aed;
}

.ph-glow--2 {
  width: 24rem;
  height: 24rem;
  top: 18rem;
  right: -6rem;
  background: #c026d3;
  opacity: 0.35;
}

/* ── Héros ── */

.ph-hero {
  position: relative;
  text-align: center;
  max-width: 44rem;
}

.ph-logo {
  width: min(250px, 26vh, 70vw);
  height: auto;
  /* Le logo est fourni sur fond noir : le halo l'intègre au dégradé. */
  filter: drop-shadow(0 0 40px rgba(168, 85, 247, 0.55));
}

.ph-tagline {
  margin: clamp(0.5rem, 1.5vh, 1rem) 0 0;
  font-size: clamp(1rem, 2.2vh, 1.25rem);
  color: #d8c7f5;
}

.ph-stats {
  display: inline-flex;
  align-items: center;
  gap: 0.6rem;
  margin-top: clamp(0.5rem, 1.5vh, 1rem);
  padding: 0.35rem 0.9rem;
  border-radius: 999px;
  background: rgba(168, 85, 247, 0.12);
  border: 1px solid rgba(168, 85, 247, 0.35);
  font-size: 0.95rem;
  color: #e9dcff;
}

.ph-stats strong {
  color: #fff;
}

.ph-guild-icon {
  width: 26px;
  height: 26px;
  border-radius: 50%;
}

.ph-actions {
  margin-top: clamp(0.75rem, 2vh, 1.5rem);
}

.ph-btn {
  display: inline-block;
  padding: clamp(0.6rem, 1.4vh, 0.85rem) 2.2rem;
  border-radius: 999px;
  font-weight: 600;
  text-decoration: none;
  transition: transform 0.15s ease, box-shadow 0.15s ease;
}

.ph-btn.primary {
  background: linear-gradient(135deg, #a855f7, #7c3aed);
  color: #fff;
  box-shadow: 0 8px 30px rgba(124, 58, 237, 0.5);
}

.ph-btn.primary:hover {
  transform: translateY(-2px);
  box-shadow: 0 12px 38px rgba(168, 85, 247, 0.65);
}

/* ── Présentation ── */

.ph-about {
  position: relative;
  max-width: 44rem;
  text-align: center;
}

.ph-about h2 {
  margin: 0 0 clamp(0.4rem, 1.2vh, 0.85rem);
  font-size: clamp(1.25rem, 3vh, 1.75rem);
}

.ph-about p {
  margin: 0;
  line-height: 1.6;
  font-size: clamp(0.9rem, 1.9vh, 1rem);
  color: #cbb8ec;
}

/* ── Piliers ── */

.ph-piliers {
  position: relative;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
  gap: clamp(0.6rem, 1.6vh, 1.1rem);
  width: 100%;
  max-width: 62rem;
}

.ph-card {
  padding: clamp(0.85rem, 2vh, 1.35rem);
  border-radius: 1rem;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(168, 85, 247, 0.22);
  backdrop-filter: blur(6px);
  transition: transform 0.15s ease, border-color 0.15s ease;
}

.ph-card:hover {
  transform: translateY(-3px);
  border-color: rgba(168, 85, 247, 0.55);
}

.ph-card-emoji {
  font-size: clamp(1.3rem, 2.8vh, 1.7rem);
}

.ph-card h3 {
  margin: 0.4rem 0 0.3rem;
  font-size: clamp(0.98rem, 2vh, 1.08rem);
}

.ph-card p {
  margin: 0;
  font-size: clamp(0.82rem, 1.7vh, 0.92rem);
  line-height: 1.5;
  color: #c3aee6;
}

/* ── Appel final ── */

.ph-footer {
  position: relative;
}

.ph-footer a {
  color: #9d84c4;
  font-size: 0.88rem;
}

.ph-footer a:hover {
  color: #d8c7f5;
}

/* Respecte le réglage système « animations réduites ». */
@media (prefers-reduced-motion: reduce) {
  .ph-btn,
  .ph-card {
    transition: none;
  }

  .ph-btn.primary:hover,
  .ph-card:hover {
    transform: none;
  }
}
</style>
