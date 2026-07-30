<script setup lang="ts">
// Accueil PUBLIC du site communautaire — visible sans connexion.
//
// Cette page est le point d'entree du site pour un visiteur anonyme. Elle est
// rendue hors de `MainLayout` (pas de barre laterale, pas de selecteur de
// serveur) : c'est une vitrine, pas un back-office.
//
// Elle n'appelle QUE `/api/public/*`, qui vit en dehors de la pile
// d'authentification. Aucun token n'est envoye, aucune donnee personnelle
// n'est demandee.
//
// Le serveur mis en avant vient de VITE_PUBLIC_GUILD_ID (build). Sans cette
// variable, la page reste valide : seul le bloc vitrine disparait.

import { onMounted, ref } from "vue";
import {
  guildIconUrl,
  publicSiteService,
  type PublicGuild,
} from "@/services/publicSiteService";

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
    // Vitrine indisponible : on n'affiche pas d'erreur technique a un
    // visiteur, la page garde son sens sans ce bloc.
    guild.value = null;
  }
});
</script>

<template>
  <div class="ph">
    <header class="ph-hero">
      <img v-if="iconUrl" :src="iconUrl" :alt="guild?.name ?? ''" class="ph-icon" />
      <h1>{{ guild?.name ?? "Notre communaute" }}</h1>
      <p v-if="guild" class="ph-members">
        {{ guild.member_count.toLocaleString("fr-FR") }} membres
      </p>
      <p class="ph-tagline">
        Evenements, jeux, classements et bien plus. Rejoins-nous.
      </p>
      <div class="ph-actions">
        <RouterLink class="ph-btn primary" to="/login">Se connecter</RouterLink>
      </div>
    </header>

    <section class="ph-teasers">
      <article class="ph-card">
        <h2>Evenements</h2>
        <p>Les prochains rendez-vous de la communaute.</p>
        <span class="ph-soon">Bientot</span>
      </article>
      <article class="ph-card">
        <h2>Concours</h2>
        <p>Les giveaways en cours et leurs gagnants.</p>
        <span class="ph-soon">Bientot</span>
      </article>
      <article class="ph-card">
        <h2>Classements</h2>
        <p>Les membres les plus actifs du serveur.</p>
        <span class="ph-soon">Bientot</span>
      </article>
    </section>

    <footer class="ph-footer">
      <RouterLink to="/login">Espace membre et administration</RouterLink>
    </footer>
  </div>
</template>

<style scoped>
.ph {
  min-height: 100vh;
  background: var(--bg-primary);
  color: var(--text-primary);
  padding: var(--space-lg);
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: var(--space-lg);
}

.ph-hero {
  text-align: center;
  max-width: 42rem;
  padding-top: var(--space-lg);
}

.ph-icon {
  width: 96px;
  height: 96px;
  border-radius: 50%;
  margin-bottom: var(--space-md);
}

.ph-hero h1 {
  margin: 0 0 var(--space-xs);
  font-size: 2.2rem;
}

.ph-members {
  color: var(--accent);
  margin: 0 0 var(--space-sm);
  font-weight: 600;
}

.ph-tagline {
  color: var(--text-secondary);
  margin: 0 0 var(--space-md);
}

.ph-actions {
  display: flex;
  justify-content: center;
  gap: var(--space-sm);
}

.ph-btn {
  display: inline-block;
  padding: var(--space-xs) var(--space-lg);
  border-radius: var(--radius-md);
  text-decoration: none;
  transition: var(--transition-fast);
}

.ph-btn.primary {
  background: var(--accent);
  color: #fff;
}

.ph-btn.primary:hover {
  background: var(--accent-hover);
}

.ph-teasers {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr));
  gap: var(--space-md);
  width: 100%;
  max-width: 60rem;
}

.ph-card {
  background: var(--bg-card);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-md);
  padding: var(--space-md);
}

.ph-card h2 {
  margin: 0 0 var(--space-xs);
  font-size: 1.1rem;
}

.ph-card p {
  color: var(--text-secondary);
  margin: 0 0 var(--space-sm);
  font-size: 0.92rem;
}

.ph-soon {
  font-size: 0.78rem;
  color: var(--text-secondary);
  border: 1px solid var(--bg-hover);
  border-radius: var(--radius-sm);
  padding: 2px 8px;
}

.ph-footer {
  margin-top: auto;
  padding-top: var(--space-lg);
}

.ph-footer a {
  color: var(--text-secondary);
  font-size: 0.88rem;
}
</style>
