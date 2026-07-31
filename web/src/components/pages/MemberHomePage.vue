<script setup lang="ts">
// Espace MEMBRE — accueil des membres connectés, distinct du back-office.
//
// Rendu hors de `MainLayout` : un membre n'a rien à faire dans la barre
// latérale d'administration, et n'y verrait de toute façon aucune entrée.
//
// Limite connue : l'API n'autorise aujourd'hui que les comptes explicitement
// invités (middleware de liste blanche). Un membre lambda du serveur peut donc
// se connecter mais n'obtiendra pas encore de données personnalisées. Ouvrir
// cet espace à tous les membres demande un niveau de rôle supplémentaire,
// en dessous des rôles d'administration actuels.

import { computed } from "vue";
import { useAuth } from "../../composables/useAuth";
import { useComponentVisibility } from "../../composables/useComponentVisibility";
import { COMMUNITY, onLogoError } from "@/branding";

const { user, avatarUrl, logout } = useAuth();
const { visible } = useComponentVisibility();

/// Le lien vers l'administration n'apparaît que pour qui y a réellement accès.
const hasAdminAccess = computed(() => visible("general.stats"));

const RUBRIQUES = [
  {
    emoji: "🗓️",
    titre: "Événements",
    texte: "Les prochains rendez-vous de la communauté et les inscriptions.",
  },
  {
    emoji: "🎁",
    titre: "Concours",
    texte: "Les giveaways en cours, les conditions et les gagnants.",
  },
  {
    emoji: "🏆",
    titre: "Classements",
    texte: "Ton rang, tes statistiques d'activité et le top du serveur.",
  },
  {
    emoji: "🎮",
    titre: "Serveurs de jeu",
    texte: "Les serveurs ouverts du moment et comment s'y connecter.",
  },
];
</script>

<template>
  <div class="mb">
    <div class="mb-glow" aria-hidden="true"></div>

    <header class="mb-header">
      <RouterLink to="/" class="mb-brand">
        <img :src="COMMUNITY.mark" :alt="COMMUNITY.name" @error="onLogoError" />
      </RouterLink>

      <div class="mb-user">
        <img v-if="avatarUrl" :src="avatarUrl" alt="" class="mb-avatar" />
        <span>{{ user?.username ?? "Membre" }}</span>
        <button type="button" class="mb-logout" @click="logout">Déconnexion</button>
      </div>
    </header>

    <section class="mb-hero">
      <h1>Salut {{ user?.username ?? "" }}&nbsp;!</h1>
      <p>Bienvenue dans ton espace membre de {{ COMMUNITY.name }}.</p>
    </section>

    <section class="mb-grid">
      <article v-for="r in RUBRIQUES" :key="r.titre" class="mb-card">
        <span class="mb-card-emoji" aria-hidden="true">{{ r.emoji }}</span>
        <h2>{{ r.titre }}</h2>
        <p>{{ r.texte }}</p>
        <span class="mb-soon">Bientôt</span>
      </article>
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
  padding: clamp(1rem, 3vh, 2rem) clamp(1rem, 4vw, 3rem);
  background: radial-gradient(circle at 50% 0%, #241040 0%, #120821 45%, #0a0512 100%);
  color: #f3eaff;
  display: flex;
  flex-direction: column;
  gap: clamp(1.25rem, 3vh, 2.25rem);
}

.mb-glow {
  position: absolute;
  top: -10rem;
  left: 50%;
  transform: translateX(-50%);
  width: 34rem;
  height: 34rem;
  border-radius: 50%;
  background: #7c3aed;
  filter: blur(110px);
  opacity: 0.4;
  pointer-events: none;
}

.mb-header {
  position: relative;
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

.mb-logout {
  background: none;
  border: 1px solid rgba(168, 85, 247, 0.35);
  color: #cbb8ec;
  border-radius: 999px;
  padding: 0.25rem 0.85rem;
  cursor: pointer;
  transition: border-color 0.15s ease, color 0.15s ease;
}

.mb-logout:hover {
  border-color: rgba(168, 85, 247, 0.8);
  color: #fff;
}

.mb-hero {
  position: relative;
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

.mb-grid {
  position: relative;
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(14rem, 1fr));
  gap: clamp(0.75rem, 2vh, 1.25rem);
  width: 100%;
  max-width: 64rem;
  margin: 0 auto;
}

.mb-card {
  padding: clamp(1rem, 2.2vh, 1.5rem);
  border-radius: 1rem;
  background: rgba(255, 255, 255, 0.04);
  border: 1px solid rgba(168, 85, 247, 0.22);
  transition: transform 0.15s ease, border-color 0.15s ease;
}

.mb-card:hover {
  transform: translateY(-3px);
  border-color: rgba(168, 85, 247, 0.55);
}

.mb-card-emoji {
  font-size: 1.6rem;
}

.mb-card h2 {
  margin: 0.45rem 0 0.35rem;
  font-size: 1.05rem;
}

.mb-card p {
  margin: 0 0 0.75rem;
  font-size: 0.9rem;
  line-height: 1.5;
  color: #c3aee6;
}

.mb-soon {
  font-size: 0.72rem;
  color: #b49ad8;
  border: 1px solid rgba(168, 85, 247, 0.3);
  border-radius: 999px;
  padding: 2px 10px;
}

.mb-footer {
  position: relative;
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
  .mb-card {
    transition: none;
  }

  .mb-card:hover {
    transform: none;
  }
}
</style>
