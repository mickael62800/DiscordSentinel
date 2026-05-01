import type { RbacRole } from "@/types";

/**
 * Registry des composants/boutons UI gardables par role applicatif.
 *
 * Chaque entree definit :
 *   - key : identifiant stable utilise en BDD (ne pas renommer apres deploy)
 *   - label : libelle affiche dans la grille de configuration
 *   - category : regroupement dans la grille
 *   - minRole : role MINIMAL par defaut pour voir le composant
 *
 * Comportement :
 *   - Si pas d'override en BDD pour (role, key) -> on applique : role >= minRole.
 *   - Si override en BDD -> il prevaut (true=visible, false=cache).
 *   - Le superadmin (SUPERADMIN_USER_IDS) bypass tout et voit tout.
 *   - Owner ne peut JAMAIS etre cache (la grille de config force visible=true pour Owner).
 */
export interface ComponentDef {
  key: string;
  label: string;
  category: string;
  minRole: RbacRole;
}

export const ROLE_RANK: Record<RbacRole, number> = {
  viewer: 0,
  moderator: 1,
  admin: 2,
  owner: 3,
};

export const ROLES_ORDER: RbacRole[] = ["viewer", "moderator", "admin", "owner"];

export const COMPONENT_REGISTRY: ComponentDef[] = [
  // ── Dashboard (clés alignees sur DashboardPage.vue) ──
  { key: "general.stats", label: "Bouton Statistiques", category: "Dashboard", minRole: "viewer" },
  { key: "general.modstats", label: "Bouton Stats modération", category: "Dashboard", minRole: "moderator" },
  { key: "moderation.hub", label: "Bouton Modération", category: "Dashboard", minRole: "moderator" },
  { key: "security.hub", label: "Bouton Sécurité", category: "Dashboard", minRole: "admin" },
  { key: "security.audit", label: "Bouton Audit", category: "Dashboard", minRole: "moderator" },
  { key: "system.settings", label: "Bouton Paramètres", category: "Dashboard", minRole: "admin" },
  { key: "system.rbac", label: "Bouton RBAC", category: "Dashboard", minRole: "owner" },
  { key: "system.systemops", label: "Bouton SystemOps", category: "Dashboard", minRole: "admin" },
  { key: "system.server_health", label: "Bouton État serveur", category: "Dashboard", minRole: "admin" },
  { key: "config.server-security", label: "Bouton Sécurité serveur", category: "Dashboard", minRole: "admin" },
  { key: "system.members", label: "Bouton Membres", category: "Dashboard", minRole: "moderator" },
  { key: "config.ai-dataset", label: "Bouton Dataset IA", category: "Dashboard", minRole: "admin" },

  // ── Dashboard : modération (sous-pages) ──
  // Strikes / Notes / Rappels / Preuves / Reviews ont ete fusionnes
  // dans /moderation (onglets Suivi utilisateur + Workflow).
  { key: "moderation.rules", label: "Bouton Règles", category: "Dashboard", minRole: "moderator" },
  { key: "moderation.members", label: "Bouton Membres", category: "Dashboard", minRole: "moderator" },
  { key: "moderation.name-history", label: "Bouton Historique noms", category: "Dashboard", minRole: "moderator" },

  // ── Dashboard : sécurité ──
  { key: "security.automod", label: "Bouton Automod", category: "Dashboard", minRole: "admin" },

  // ── Dashboard : community ──
  { key: "community.welcome", label: "Bouton Welcome", category: "Dashboard", minRole: "admin" },
  { key: "community.tickets", label: "Bouton Tickets", category: "Dashboard", minRole: "moderator" },
  { key: "community.voice-channels", label: "Bouton Salons vocaux", category: "Dashboard", minRole: "moderator" },
  { key: "community.voice-themes", label: "Bouton Thèmes vocaux", category: "Dashboard", minRole: "admin" },
  { key: "community.role-panels", label: "Bouton Role panels", category: "Dashboard", minRole: "admin" },
  { key: "community.levels", label: "Bouton Niveaux", category: "Dashboard", minRole: "viewer" },
  { key: "community.levels-config", label: "Bouton Config niveaux", category: "Dashboard", minRole: "admin" },
  { key: "community.sponsorships", label: "Bouton Parrainages", category: "Dashboard", minRole: "moderator" },
  { key: "community.temp-roles", label: "Bouton Rôles temporaires", category: "Dashboard", minRole: "moderator" },

  // ── Dashboard : logs / jeux ──
  { key: "logs.journal", label: "Bouton Journal logs Discord", category: "Dashboard", minRole: "admin" },
  { key: "logs.system", label: "Bouton Logs système (bots/workers/API/WS)", category: "Dashboard", minRole: "admin" },
  { key: "games.hub", label: "Bouton Jeux", category: "Dashboard", minRole: "viewer" },
  { key: "games.coude", label: "Bouton Coude (hub : stats / social / tournoi)", category: "Dashboard", minRole: "viewer" },
  { key: "games.blackjack", label: "Bouton Blackjack", category: "Dashboard", minRole: "viewer" },
  { key: "games.slot", label: "Bouton Slot", category: "Dashboard", minRole: "viewer" },
  { key: "games.wheel", label: "Bouton Roue", category: "Dashboard", minRole: "viewer" },
  { key: "games.wallet", label: "Bouton Wallet", category: "Dashboard", minRole: "viewer" },
  { key: "games.taunts", label: "Bouton Railleries (Coude + Blackjack)", category: "Dashboard", minRole: "admin" },

  // ── Dashboard : config (admin) ──
  { key: "config.components", label: "Bouton Config composants", category: "Dashboard", minRole: "admin" },
  { key: "config.rbac", label: "Bouton Config RBAC", category: "Dashboard", minRole: "owner" },
  { key: "config.system-ops", label: "Bouton System Ops", category: "Dashboard", minRole: "admin" },
  { key: "config.server-health", label: "Bouton État serveur (config)", category: "Dashboard", minRole: "admin" },
  { key: "config.settings", label: "Bouton Paramètres (config)", category: "Dashboard", minRole: "admin" },

  // ── Docker ──
  { key: "docker.section", label: "Section Docker (visibilité)", category: "Docker", minRole: "admin" },
  { key: "docker.action.start", label: "Démarrer un conteneur", category: "Docker", minRole: "owner" },
  { key: "docker.action.stop", label: "Arrêter un conteneur", category: "Docker", minRole: "owner" },
  { key: "docker.action.restart", label: "Redémarrer un conteneur", category: "Docker", minRole: "owner" },
  { key: "docker.action.remove_container", label: "Supprimer un conteneur", category: "Docker", minRole: "owner" },
  { key: "docker.action.remove_image", label: "Supprimer une image", category: "Docker", minRole: "owner" },
  { key: "docker.action.remove_volume", label: "Supprimer un volume", category: "Docker", minRole: "owner" },
  { key: "docker.action.logs", label: "Voir les logs", category: "Docker", minRole: "admin" },
  { key: "docker.prune.containers", label: "Nettoyage : conteneurs", category: "Docker", minRole: "owner" },
  { key: "docker.prune.images", label: "Nettoyage : images", category: "Docker", minRole: "owner" },
  { key: "docker.prune.volumes", label: "Nettoyage : volumes", category: "Docker", minRole: "owner" },
  { key: "docker.prune.networks", label: "Nettoyage : réseaux", category: "Docker", minRole: "owner" },
  { key: "docker.prune.system", label: "Nettoyage système complet", category: "Docker", minRole: "owner" },

  // ── Modération ──
  { key: "moderation.purge", label: "Purge messages", category: "Modération", minRole: "moderator" },
  { key: "moderation.bulk_unban", label: "Débannissement en masse", category: "Modération", minRole: "admin" },
  { key: "moderation.delete_action", label: "Supprimer action", category: "Modération", minRole: "admin" },

  // ── Membres ──
  { key: "members.reset", label: "Reset complet d'un membre", category: "Membres", minRole: "owner" },
  { key: "members.surveillance", label: "Onglet Surveillance", category: "Membres", minRole: "moderator" },

  // ── RBAC ──
  { key: "rbac.grant", label: "Attribuer un rôle", category: "RBAC", minRole: "owner" },
  { key: "rbac.revoke", label: "Révoquer un rôle", category: "RBAC", minRole: "owner" },
  { key: "rbac.visibility", label: "Configurer visibilité composants", category: "RBAC", minRole: "owner" },

  // ── Exports ──
  { key: "exports.create", label: "Créer un export", category: "Exports", minRole: "moderator" },

  // ── Composants bots ──
  { key: "components.toggle", label: "Activer/désactiver un composant", category: "Configuration", minRole: "admin" },
];

export function componentByKey(key: string): ComponentDef | undefined {
  return COMPONENT_REGISTRY.find((c) => c.key === key);
}

export function categories(): string[] {
  const set = new Set<string>();
  for (const c of COMPONENT_REGISTRY) set.add(c.category);
  return Array.from(set);
}
