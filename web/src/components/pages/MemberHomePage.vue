<script setup lang="ts">
// Espace membre — la vie du serveur, consultable SANS connexion.
//
// Choix structurant : cette page est publique. Un visiteur doit pouvoir voir
// ce qui se passe — le planning, ce qui est en cours — avant de décider de
// créer un compte. Demander la connexion à l'entrée revenait à mettre un
// videur devant une vitrine.
//
// La connexion n'est requise que pour AGIR : s'inscrire à un événement,
// retrouver ses stats. Les boutons d'action affichent alors une invitation à
// se connecter plutôt que de disparaître — l'utilisateur comprend ce qu'il
// gagnerait à le faire.
//
// Rendue hors de `MainLayout` : un membre n'a rien à faire dans la barre
// latérale d'administration.

import { computed, onMounted, ref } from "vue";
import { useAuth } from "../../composables/useAuth";
import { useComponentVisibility } from "../../composables/useComponentVisibility";
import { COMMUNITY, onLogoError } from "@/branding";
import {
  isOngoing,
  publicEventsService,
  type PublicEvent,
} from "@/services/publicEventsService";
import CommunityBackdrop from "../atoms/CommunityBackdrop.vue";

const guildId = import.meta.env.VITE_PUBLIC_GUILD_ID as string | undefined;

const { user, avatarUrl, logout } = useAuth();
const { visible } = useComponentVisibility();

/// Le lien vers l'administration n'apparaît que pour qui y a réellement accès.
const hasAdminAccess = computed(() => visible("general.stats"));

const events = ref<PublicEvent[]>([]);
const loadingEvents = ref(true);

onMounted(async () => {
  if (!guildId) {
    loadingEvents.value = false;
    return;
  }
  // Fenêtre large : ce qui est en cours (commencé avant aujourd'hui) et ce qui
  // arrive dans les deux mois.
  const from = new Date();
  from.setDate(from.getDate() - 30);
  const to = new Date();
  to.setDate(to.getDate() + 60);

  try {
    events.value = await publicEventsService.list(guildId, from, to);
  } catch {
    events.value = [];
  } finally {
    loadingEvents.value = false;
  }
});

/// En cours d'abord, puis à venir par ordre chronologique.
const ongoing = computed(() => events.value.filter((e) => isOngoing(e)));
const upcoming = computed(() => {
  const now = new Date();
  return events.value
    .filter((e) => new Date(e.starts_at) > now)
    .sort((a, b) => a.starts_at.localeCompare(b.starts_at))
    .slice(0, 6);
});

function fmtRange(e: PublicEvent): string {
  const start = new Date(e.starts_at);
  const end = new Date(e.ends_at);
  const jour: Intl.DateTimeFormatOptions = { weekday: "short", day: "numeric", month: "short" };
  const heure: Intl.DateTimeFormatOptions = { hour: "2-digit", minute: "2-digit" };

  // Une campagne s'annonce par ses dates, une soirée par son horaire : afficher
  // « 21:00 » pour un événement de trois semaines n'aurait aucun sens.
  if (e.span_days > 1) {
    return `${start.toLocaleDateString("fr-FR", jour)} → ${end.toLocaleDateString("fr-FR", jour)}`;
  }
  if (e.all_day) return start.toLocaleDateString("fr-FR", jour);
  return `${start.toLocaleDateString("fr-FR", jour)} · ${start.toLocaleTimeString("fr-FR", heure)}`;
}

function accent(e: PublicEvent): string | undefined {
  return e.color ? `#${e.color}` : undefined;
}

const RUBRIQUES = [
  { emoji: "🎁", titre: "Concours", texte: "Les giveaways en cours et les gagnants." },
  { emoji: "🏆", titre: "Classements", texte: "Ton rang et le top du serveur." },
  { emoji: "🎮", titre: "Serveurs de jeu", texte: "Ce qui tourne et comment s'y connecter." },
];
</script>

<template>
  <div class="mb">
    <CommunityBackdrop :intensity="0.8" />

    <header class="mb-header">
      <RouterLink to="/" class="mb-brand">
        <img :src="COMMUNITY.mark" :alt="COMMUNITY.name" @error="onLogoError" />
      </RouterLink>

      <div v-if="user" class="mb-user">
        <img v-if="avatarUrl" :src="avatarUrl" alt="" class="mb-avatar" />
        <span>{{ user.username }}</span>
        <button type="button" class="mb-ghost" @click="logout">Déconnexion</button>
      </div>
      <RouterLink v-else to="/login?espace=membre" class="mb-ghost">Se connecter</RouterLink>
    </header>

    <section class="mb-hero">
      <h1 v-if="user">Salut {{ user.username }}&nbsp;!</h1>
      <h1 v-else>La vie du serveur</h1>
      <p v-if="user">Voici ce qui se passe chez {{ COMMUNITY.name }}.</p>
      <p v-else>
        Ce qui se passe en ce moment chez {{ COMMUNITY.name }}. Connecte-toi pour
        t'inscrire aux événements.
      </p>
    </section>

    <!-- ── En cours ── -->
    <section v-if="ongoing.length" class="mb-block">
      <h2><span class="mb-live" aria-hidden="true"></span> En ce moment</h2>
      <ul class="mb-events">
        <li
          v-for="e in ongoing"
          :key="e.id"
          class="mb-event ongoing"
          :style="{ '--accent-event': accent(e) }"
        >
          <div class="mb-event-main">
            <strong>{{ e.title }}</strong>
            <span v-if="e.game" class="mb-tag">{{ e.game }}</span>
          </div>
          <p v-if="e.description" class="mb-event-desc">{{ e.description }}</p>
          <span class="mb-event-when">Jusqu'au {{ fmtRange(e).split("→").pop()?.trim() }}</span>
        </li>
      </ul>
    </section>

    <!-- ── À venir ── -->
    <section class="mb-block">
      <h2>Prochainement</h2>

      <p v-if="loadingEvents" class="mb-hint">Chargement du planning…</p>
      <p v-else-if="!upcoming.length" class="mb-hint">
        Rien de prévu pour l'instant. Ça ne veut pas dire qu'il ne se passe rien
        sur le vocal&nbsp;!
      </p>

      <ul v-else class="mb-events">
        <li
          v-for="e in upcoming"
          :key="e.id"
          class="mb-event"
          :style="{ '--accent-event': accent(e) }"
        >
          <div class="mb-event-main">
            <strong>{{ e.title }}</strong>
            <span v-if="e.game" class="mb-tag">{{ e.game }}</span>
            <span v-if="e.span_days > 1" class="mb-tag long">{{ e.span_days }} jours</span>
          </div>
          <p v-if="e.description" class="mb-event-desc">{{ e.description }}</p>
          <div class="mb-event-foot">
            <span class="mb-event-when">{{ fmtRange(e) }}</span>
            <!-- L'inscription arrive : le bouton explique déjà à quoi sert le
                 compte, plutôt que de cacher la fonctionnalité. -->
            <RouterLink v-if="!user" to="/login?espace=membre" class="mb-join">
              Se connecter pour s'inscrire
            </RouterLink>
            <span v-else class="mb-soon">Inscription bientôt</span>
          </div>
        </li>
      </ul>
    </section>

    <!-- ── Rubriques à venir ── -->
    <section class="mb-block">
      <h2>Bientôt sur le site</h2>
      <div class="mb-grid">
        <article v-for="r in RUBRIQUES" :key="r.titre" class="mb-card">
          <span class="mb-card-emoji" aria-hidden="true">{{ r.emoji }}</span>
          <h3>{{ r.titre }}</h3>
          <p>{{ r.texte }}</p>
        </article>
      </div>
    </section>

    <footer class="mb-footer">
      <RouterLink v-if="hasAdminAccess" to="/dashboard" class="mb-admin-link">
        🛡️ Accéder à l'administration
      </RouterLink>
    </footer>
  </div>
</template>

<style scoped>
.mb {
  flex: 1;
  position: relative;
  overflow-x: hidden;
  overflow-y: auto;
  padding: clamp(1rem, 3vh, 2rem) clamp(1rem, 4vw, 3rem) 3rem;
  background: linear-gradient(180deg, #150a28 0%, #0d0619 55%, #08040f 100%);
  color: #f3eaff;
  display: flex;
  flex-direction: column;
  gap: clamp(1.5rem, 4vh, 2.5rem);
}

.mb-header,
.mb-hero,
.mb-block,
.mb-footer {
  position: relative;
  z-index: 1;
  width: 100%;
  max-width: 60rem;
  margin: 0 auto;
}

.mb-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 1rem;
}

.mb-brand img {
  width: 46px;
  height: 46px;
  display: block;
}

.mb-user {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  font-size: 0.92rem;
  color: #d8c7f5;
}

.mb-avatar {
  width: 30px;
  height: 30px;
  border-radius: 50%;
}

.mb-ghost {
  background: none;
  border: 1px solid rgba(168, 85, 247, 0.35);
  color: #cbb8ec;
  border-radius: 999px;
  padding: 0.3rem 0.95rem;
  font-size: 0.88rem;
  cursor: pointer;
  transition: border-color 0.15s ease, color 0.15s ease;
}

.mb-ghost:hover {
  border-color: rgba(168, 85, 247, 0.85);
  color: #fff;
}

.mb-hero {
  text-align: center;
  margin-top: clamp(0.5rem, 2vh, 1.5rem);
}

.mb-hero h1 {
  margin: 0 0 0.4rem;
  font-size: clamp(1.5rem, 4vh, 2.2rem);
}

.mb-hero p {
  margin: 0;
  color: #cbb8ec;
}

.mb-block h2 {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  margin: 0 0 0.9rem;
  font-size: 1.15rem;
}

/* Pastille « en direct » : discrète, elle pulse doucement. */
.mb-live {
  width: 9px;
  height: 9px;
  border-radius: 50%;
  background: #22c55e;
  box-shadow: 0 0 10px #22c55e;
  animation: pulse 2.2s ease-in-out infinite;
}

@keyframes pulse {
  50% {
    opacity: 0.35;
  }
}

.mb-hint {
  color: #b49ad8;
  margin: 0;
}

.mb-events {
  list-style: none;
  margin: 0;
  padding: 0;
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
}

.mb-event {
  --accent-event: #a855f7;
  padding: 0.9rem 1.1rem;
  border-radius: 0.9rem;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(168, 85, 247, 0.2);
  border-left: 3px solid var(--accent-event);
}

.mb-event.ongoing {
  background: rgba(34, 197, 94, 0.07);
  border-color: rgba(34, 197, 94, 0.25);
}

.mb-event-main {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 0.5rem;
}

.mb-tag {
  font-size: 0.74rem;
  padding: 1px 9px;
  border-radius: 999px;
  background: rgba(168, 85, 247, 0.16);
  color: #d8c7f5;
}

.mb-tag.long {
  background: rgba(255, 255, 255, 0.08);
}

.mb-event-desc {
  margin: 0.35rem 0 0;
  font-size: 0.88rem;
  line-height: 1.5;
  color: #c3aee6;
}

.mb-event-foot {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  margin-top: 0.55rem;
}

.mb-event-when {
  display: inline-block;
  font-size: 0.82rem;
  color: #b49ad8;
}

.mb-join {
  font-size: 0.82rem;
  color: #d8c7f5;
  border-bottom: 1px dotted rgba(216, 199, 245, 0.5);
}

.mb-join:hover {
  color: #fff;
}

.mb-soon {
  font-size: 0.74rem;
  color: #b49ad8;
  border: 1px solid rgba(168, 85, 247, 0.3);
  border-radius: 999px;
  padding: 2px 10px;
}

.mb-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
  gap: 0.9rem;
}

.mb-card {
  padding: 1.1rem;
  border-radius: 0.9rem;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(168, 85, 247, 0.2);
}

.mb-card-emoji {
  font-size: 1.5rem;
}

.mb-card h3 {
  margin: 0.4rem 0 0.3rem;
  font-size: 1rem;
}

.mb-card p {
  margin: 0;
  font-size: 0.87rem;
  line-height: 1.5;
  color: #c3aee6;
}

.mb-footer {
  margin-top: auto;
  padding-top: 1rem;
  text-align: center;
}

.mb-admin-link {
  color: #9d84c4;
  font-size: 0.9rem;
}

.mb-admin-link:hover {
  color: #d8c7f5;
}

@media (prefers-reduced-motion: reduce) {
  .mb-live {
    animation: none;
  }
}
</style>
