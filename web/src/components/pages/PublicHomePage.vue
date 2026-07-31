<script setup lang="ts">
// Accueil PUBLIC du site communautaire — visible sans connexion.
//
// Page longue, assumée : un visiteur doit comprendre ce qu'est La Bande du
// Canapé, ce qu'on y fait, et avoir envie d'entrer. Le défilement est donc
// normal ici — contrairement au reste du site, elle raconte quelque chose.
//
// Rendue hors de `MainLayout` (ni barre latérale, ni sélecteur de serveur) :
// c'est une vitrine, pas un back-office. Elle n'appelle QUE `/api/public/*`,
// hors de la pile d'authentification : aucun token, aucune donnée personnelle.
//
// Images : chaque section en accepte une, déclarée dans SECTIONS. Tant qu'un
// fichier est absent, `onIllustrationError` masque proprement le bloc et la
// section reste lisible en pleine largeur.

import { onMounted, ref } from "vue";
import {
  guildIconUrl,
  publicSiteService,
  type PublicGuild,
} from "@/services/publicSiteService";
import { COMMUNITY, onWordmarkError, wordmarkOf } from "@/branding";
import CommunityBackdrop from "../atoms/CommunityBackdrop.vue";

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

/// Masque l'illustration si le fichier n'est pas encore fourni : la section
/// bascule alors en pleine largeur au lieu d'afficher une image cassée.
function onIllustrationError(event: Event): void {
  const el = event.target as HTMLImageElement | null;
  el?.closest(".sec-media")?.setAttribute("hidden", "true");
}

/// Sections alternées : l'illustration passe à droite puis à gauche.
const SECTIONS = [
  {
    id: "jeux",
    surtitre: "Nos serveurs",
    titre: "On monte les serveurs, tu joues",
    texte:
      "Minecraft, Palworld et d'autres : nos serveurs se créent en quelques clics et tournent sur notre propre machine. Pas de file d'attente, pas de publicité, pas de loyer mensuel à payer pour jouer entre nous.",
    points: [
      "Serveurs montés à la demande, éteints quand personne ne joue",
      "Sauvegardes automatiques du monde",
      "Un salon Discord dédié créé pour chaque session",
    ],
    image: "/site/section-jeux.jpg",
    alt: "Serveurs de jeu de la communauté",
  },
  {
    id: "vocal",
    surtitre: "La vie du serveur",
    titre: "Des vocaux ouverts, tout le temps",
    texte:
      "Le cœur de la bande, c'est le vocal. Il y a presque toujours quelqu'un. On y parle de tout, on y joue, ou on laisse simplement tourner en fond pendant qu'on fait autre chose.",
    points: [
      "Salons vocaux créés automatiquement quand il en manque",
      "Personne n'est obligé de parler : venir écouter, ça compte",
      "Des salons privés à la demande pour les petits groupes",
    ],
    image: "/site/section-vocal.jpg",
    alt: "Salons vocaux de la communauté",
  },
  {
    id: "planning",
    surtitre: "Le planning",
    titre: "Ce qui arrive, et quand",
    texte:
      "Une soirée un mardi soir, une saison Minecraft qui dure trois semaines, une campagne Palworld sur un mois : le planning affiche les deux, en vue semaine ou en vue mois. Tu vois d'un coup d'œil ce qui tourne et ce qui se prépare.",
    points: [
      "Vue semaine pour les soirées, vue mois pour les campagnes",
      "Les événements longs restent visibles toute leur durée",
      "Inscription en un clic, avec les « peut-être » qui comptent aussi",
    ],
    image: "/site/section-planning.jpg",
    alt: "Planning des événements et campagnes",
  },
  {
    id: "animation",
    surtitre: "Concours",
    titre: "Tirages au sort et petits jeux",
    texte:
      "Des giveaways réguliers, une monnaie maison à dépenser, et quelques jeux internes qui tournent directement sur Discord. Rien d'obligatoire, tout est là pour l'ambiance — et pour le plaisir de gagner un truc de temps en temps.",
    points: [
      "Tirages au sort transparents, gagnants annoncés publiquement",
      "Une monnaie du serveur à gagner et à dépenser",
      "Des jeux internes jouables sans rien installer",
    ],
    image: "/site/section-animation.jpg",
    alt: "Concours et tirages au sort",
  },
  {
    id: "classements",
    surtitre: "Classements",
    titre: "Qui traîne vraiment le plus sur le canapé",
    texte:
      "Temps passé en vocal, messages échangés, niveaux gagnés : tout est compté, et affiché. Sans prise de tête, juste de quoi savoir qui squatte le plus et se chambrer en connaissance de cause.",
    points: [
      "Classement du temps en vocal et des messages",
      "Niveaux et expérience gagnés en participant",
      "Statistiques du serveur, mois par mois",
    ],
    image: "/site/section-classements.jpg",
    alt: "Classements de la communauté",
  },
  {
    id: "moderation",
    surtitre: "Un cadre sain",
    titre: "Modéré, sans être fliqué",
    texte:
      "On tient à ce que le canapé reste confortable. La modération est présente et outillée, mais discrète : elle intervient sur ce qui gêne réellement, pas sur les blagues.",
    points: [
      "Règles claires, affichées, appliquées de la même façon pour tous",
      "Détection automatique du spam et des raids",
      "Chaque décision est tracée et peut être contestée",
    ],
    image: "/site/section-moderation.jpg",
    alt: "Une communauté modérée",
  },
];

/// Réglages d'apparition partagés : léger décalage vers le haut, une seule
/// fois, au passage dans le champ de vision.
const APPEAR = {
  initial: { opacity: 0, y: 28 },
  visibleOnce: { opacity: 1, y: 0, transition: { duration: 550 } },
};
</script>

<template>
  <div class="ph">
    <CommunityBackdrop />

    <!-- ── Hero ── -->
    <header class="ph-hero">
      <img
        v-motion
        :initial="{ opacity: 0, scale: 0.94 }"
        :enter="{ opacity: 1, scale: 1, transition: { duration: 700 } }"
        :src="wordmarkOf(COMMUNITY)"
        :alt="COMMUNITY.name"
        class="ph-logo"
        @error="onWordmarkError($event, COMMUNITY)"
      />

      <p v-motion :initial="{ opacity: 0 }" :enter="{ opacity: 1, transition: { delay: 250 } }" class="ph-tagline">
        {{ COMMUNITY.tagline }}
      </p>

      <div v-if="guild" class="ph-stats">
        <img v-if="iconUrl" :src="iconUrl" :alt="guild.name" class="ph-guild-icon" />
        <span>
          <strong>{{ guild.member_count.toLocaleString("fr-FR") }}</strong>
          membres sur {{ guild.name }}
        </span>
      </div>

      <div class="ph-actions">
        <RouterLink class="ph-btn primary" to="/membre">
          Rejoindre la bande
        </RouterLink>
        <a class="ph-btn ghost" href="#jeux">Découvrir</a>
      </div>
    </header>

    <!-- ── Présentation ── -->
    <section v-motion="APPEAR" class="ph-about">
      <h2>C'est quoi, La Bande du Canapé&nbsp;?</h2>
      <p>
        Un serveur Discord sans prise de tête. On s'y retrouve pour jouer, pour
        discuter, ou juste pour avoir un peu de bruit de fond pendant qu'on fait
        autre chose. Pas de niveau minimum, pas d'audition&nbsp;: si tu poses tes
        fesses sur le canapé, tu fais partie de la bande.
      </p>
    </section>

    <!-- ── Sections alternées ── -->
    <section
      v-for="(s, i) in SECTIONS"
      :id="s.id"
      :key="s.id"
      v-motion="APPEAR"
      class="ph-sec"
      :class="{ reverse: i % 2 === 1 }"
    >
      <div class="sec-text">
        <span class="sec-sur">{{ s.surtitre }}</span>
        <h2>{{ s.titre }}</h2>
        <p>{{ s.texte }}</p>
        <ul>
          <li v-for="pt in s.points" :key="pt">{{ pt }}</li>
        </ul>
      </div>

      <figure class="sec-media">
        <img :src="s.image" :alt="s.alt" loading="lazy" @error="onIllustrationError" />
      </figure>
    </section>

    <!-- ── Appel final ── -->
    <section v-motion="APPEAR" class="ph-cta">
      <h2>Le canapé est large, il reste de la place</h2>
      <p>Connexion avec ton compte Discord, rien d'autre à installer.</p>
      <RouterLink class="ph-btn primary" to="/membre">Entrer</RouterLink>
    </section>

    <!-- ── Deux portes ── -->
    <section class="ph-portes">
      <RouterLink class="ph-porte" to="/membre">
        <span class="ph-porte-emoji" aria-hidden="true">🛋️</span>
        <span class="ph-porte-titre">Espace membre</span>
        <span class="ph-porte-texte">Événements, concours, classements</span>
      </RouterLink>

      <RouterLink class="ph-porte" to="/login?espace=admin">
        <span class="ph-porte-emoji" aria-hidden="true">🛡️</span>
        <span class="ph-porte-titre">Administration</span>
        <span class="ph-porte-texte">Modération, configuration, journaux</span>
      </RouterLink>
    </section>
  </div>
</template>

<style scoped>
/* Palette locale calée sur le logo (violet néon sur fond très sombre). Elle ne
   dépend pas du thème du back-office : cette page a sa propre identité. */
.ph {
  flex: 1;
  position: relative;
  /* Les halos débordent volontairement des bords : sans ça, ils créeraient une
     barre de défilement horizontale. */
  overflow-x: hidden;
  overflow-y: auto;
  padding: clamp(2rem, 6vh, 4rem) 1.5rem clamp(2rem, 5vh, 4rem);
  background: linear-gradient(180deg, #150a28 0%, #0d0619 55%, #08040f 100%);
  color: #f3eaff;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: clamp(3rem, 9vh, 6rem);
  scroll-behavior: smooth;
}

/* ── Hero ── */

.ph-hero {
  position: relative;
  z-index: 1;
  text-align: center;
  max-width: 44rem;
  padding-top: clamp(1rem, 4vh, 3rem);
}

.ph-logo {
  width: min(320px, 72vw);
  height: auto;
  filter: drop-shadow(0 0 45px rgba(168, 85, 247, 0.55));
}

.ph-tagline {
  margin: 1.5rem 0 0;
  font-size: clamp(1.05rem, 2.4vw, 1.35rem);
  color: #d8c7f5;
}

.ph-stats {
  display: inline-flex;
  align-items: center;
  gap: 0.6rem;
  margin-top: 1.25rem;
  padding: 0.4rem 1rem;
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
  margin-top: 2rem;
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.75rem;
}

.ph-btn {
  display: inline-block;
  padding: 0.85rem 2.2rem;
  border-radius: 999px;
  font-weight: 600;
  text-decoration: none;
  transition: transform 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease;
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

.ph-btn.ghost {
  border: 1px solid rgba(168, 85, 247, 0.45);
  color: #d8c7f5;
}

.ph-btn.ghost:hover {
  border-color: rgba(168, 85, 247, 0.9);
  color: #fff;
}

/* ── Présentation ── */

.ph-about {
  position: relative;
  z-index: 1;
  max-width: 44rem;
  text-align: center;
}

.ph-about h2,
.ph-cta h2,
.ph-sec h2 {
  margin: 0 0 1rem;
  font-size: clamp(1.4rem, 3.2vw, 2rem);
  line-height: 1.25;
}

.ph-about p {
  margin: 0;
  line-height: 1.7;
  color: #cbb8ec;
}

/* ── Sections alternées ── */

.ph-sec {
  position: relative;
  z-index: 1;
  display: grid;
  grid-template-columns: 1fr 1fr;
  align-items: center;
  gap: clamp(1.5rem, 4vw, 3.5rem);
  width: 100%;
  max-width: 64rem;
  scroll-margin-top: 2rem;
}

/* Une section sur deux inverse texte et image. `direction` plutôt qu'un
   réordonnancement manuel : le DOM garde l'ordre de lecture. */
.ph-sec.reverse {
  direction: rtl;
}

.ph-sec.reverse > * {
  direction: ltr;
}

.sec-sur {
  display: inline-block;
  margin-bottom: 0.6rem;
  padding: 2px 12px;
  border-radius: 999px;
  background: rgba(168, 85, 247, 0.15);
  border: 1px solid rgba(168, 85, 247, 0.3);
  font-size: 0.76rem;
  letter-spacing: 0.06em;
  text-transform: uppercase;
  color: #cbb0ff;
}

.sec-text p {
  margin: 0 0 1rem;
  line-height: 1.7;
  color: #cbb8ec;
}

.sec-text ul {
  margin: 0;
  padding: 0;
  list-style: none;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.sec-text li {
  position: relative;
  padding-left: 1.5rem;
  font-size: 0.94rem;
  color: #d8c7f5;
}

.sec-text li::before {
  content: "";
  position: absolute;
  left: 0;
  top: 0.5em;
  width: 8px;
  height: 8px;
  border-radius: 50%;
  background: #a855f7;
  box-shadow: 0 0 10px #a855f7;
}

.sec-media {
  margin: 0;
  /* Arrondi genereux, dans l'esprit du logo. `overflow: hidden` recadre
     l'image elle-meme : le fichier reste un rectangle, l'arrondi est fait
     en CSS — plus net qu'un masque grave dans le JPEG, et adaptatif. */
  border-radius: 1.75rem;
  overflow: hidden;
  border: 1px solid rgba(168, 85, 247, 0.25);
  box-shadow: 0 20px 50px rgba(0, 0, 0, 0.45);
}

.sec-media img {
  display: block;
  width: 100%;
  height: auto;
  aspect-ratio: 4 / 3;
  object-fit: cover;
}

/* ── Appel final ── */

.ph-cta {
  position: relative;
  z-index: 1;
  text-align: center;
  padding: clamp(2rem, 5vw, 3rem);
  border-radius: 1.25rem;
  background: rgba(124, 58, 237, 0.12);
  border: 1px solid rgba(168, 85, 247, 0.3);
  max-width: 44rem;
  width: 100%;
}

.ph-cta p {
  margin: 0 0 1.75rem;
  color: #cbb8ec;
}

/* ── Deux portes ── */

.ph-portes {
  position: relative;
  z-index: 1;
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 1rem;
  width: 100%;
  max-width: 44rem;
}

.ph-porte {
  flex: 1 1 15rem;
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.15rem;
  padding: 1.1rem 1.25rem;
  border-radius: 0.9rem;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(168, 85, 247, 0.25);
  text-align: center;
  transition: transform 0.15s ease, border-color 0.15s ease;
}

.ph-porte:hover {
  transform: translateY(-2px);
  border-color: rgba(168, 85, 247, 0.6);
}

.ph-porte-emoji {
  font-size: 1.3rem;
}

.ph-porte-titre {
  font-weight: 600;
  color: #f3eaff;
}

.ph-porte-texte {
  font-size: 0.82rem;
  color: #b49ad8;
}

/* ── Adaptatif ── */

@media (max-width: 820px) {
  .ph-sec,
  .ph-sec.reverse {
    grid-template-columns: 1fr;
    direction: ltr;
  }

  /* Sur mobile l'illustration passe toujours après le texte. */
  .sec-media {
    order: 2;
  }
}

@media (prefers-reduced-motion: reduce) {
  .ph {
    scroll-behavior: auto;
  }

  .ph-btn,
  .ph-porte {
    transition: none;
  }

  .ph-btn.primary:hover,
  .ph-porte:hover {
    transform: none;
  }
}
</style>
